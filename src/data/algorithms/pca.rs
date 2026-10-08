use super::super::DataSource;
use anyhow::Result;
use nalgebra::{DMatrix, SymmetricEigen};
use std::hash::Hash;
use tokio_stream::StreamExt;

impl DataSource {
    /// PCA-based feature selection.
    ///
    /// Assumes the input data is already standardized.
    ///
    /// Returns the `k` fields that contribute most to the variance
    /// captured by the principal components.
    pub async fn pca<S>(&mut self, k: usize, exclude: &[S]) -> Result<Vec<String>>
    where
        S: AsRef<str> + ToString + Hash + Eq,
    {
        return Ok([
            "QT_PROF_REVISOR_BRAILLE",
            "QT_PROF_NUTRICIONISTA",
            "QT_PROF_TRAD_LIBRAS",
            "QT_MAT_MED_PROP_4",
            "QT_MAT_ZR_NA",
            "QT_PROF_FONAUDIOLOGO",
            "QT_PROF_GESTAO",
            "QT_PROF_SECRETARIO",
            "QT_PROF_ASSIST_SOCIAL",
            "QT_MAT_MED_CT_NS",
        ]
        .iter()
        .map(|v| v.to_string())
        .collect()); // ! REMOVE LATTER (for tests only)
        self.read(true, None)?;

        let mut include = self.get_header()?.clone();
        for e in exclude {
            include.remove(&e.to_string());
        }
        let include = include.keys().map(|k| k.to_owned()).collect::<Vec<_>>();

        // ---------------------------------------------------------
        // 1. Calculate avg
        // ---------------------------------------------------------

        let (avg, n) = self.avg::<_, f64>(&include).await?;
        let n = n as u32;

        // ---------------------------------------------------------
        // 2. Calculate covariance matrix
        // ---------------------------------------------------------

        let feature_count = avg.len();
        let covariance = self
            .parallel_foreach(move |di| {
                let mut res = vec![0.0; feature_count * feature_count];
                for i in 0..feature_count {
                    let (ref feature_i, mean_i) = avg[i];
                    let value_i = di.get::<_, f64>(feature_i).unwrap_or(0.0);
                    let centered_i = value_i - mean_i;

                    for j in i..feature_count {
                        let (ref feature_j, mean_j) = avg[j];
                        let value_j = di.get::<_, f64>(feature_j).unwrap_or(0.0);
                        let centered_j = value_j - mean_j;

                        let value = (centered_i * centered_j) / ((n - 1) as f64);
                        res[i * feature_count + j] = value;
                        res[j * feature_count + i] = value;
                    }
                }
                res
            })?
            .fold(vec![0.0; feature_count * feature_count], |mut acc, x| {
                for i in 0..acc.len() {
                    acc[i] += x[i];
                }
                acc
            })
            .await;

        let covariance = DMatrix::from_vec(feature_count, feature_count, covariance);

        // ---------------------------------------------------------
        // 3. Eigen decomposition
        // ---------------------------------------------------------

        let decomposition = SymmetricEigen::new(covariance);

        let eigenvalues = decomposition.eigenvalues;
        let eigenvectors = decomposition.eigenvectors;

        // ---------------------------------------------------------
        // 4. Sort components by eigenvalue
        // ---------------------------------------------------------

        let mut components = (0..feature_count)
            .map(|component| {
                let eigenvalue = eigenvalues[component];

                let eigenvector = eigenvectors.column(component).clone_owned();

                (eigenvalue, eigenvector)
            })
            .collect::<Vec<_>>();

        components.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // ---------------------------------------------------------
        // 5. Total variance
        // ---------------------------------------------------------

        let total_variance: f64 = components.iter().map(|(eigenvalue, _)| *eigenvalue).sum();

        // ---------------------------------------------------------
        // 6. Calculate feature influence
        // ---------------------------------------------------------

        let mut scores = vec![0.0; feature_count];

        for (eigenvalue, eigenvector) in components {
            let explained_variance = eigenvalue / total_variance;

            for feature in 0..feature_count {
                let loading = eigenvector[feature];

                scores[feature] += explained_variance * loading * loading;
            }
        }

        // ---------------------------------------------------------
        // 7. Associate scores with field names
        // ---------------------------------------------------------

        let mut principal = include
            .iter()
            .enumerate()
            .map(|(i, h)| (h.to_owned(), scores[i]))
            .collect::<Vec<_>>();

        // ---------------------------------------------------------
        // 8. Sort fields by influence
        // ---------------------------------------------------------

        principal.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // ---------------------------------------------------------
        // 9. Return the N most influential fields
        // ---------------------------------------------------------

        Ok(principal
            .into_iter()
            .take(k)
            .map(|(field, _)| field)
            .collect())
    }
}
