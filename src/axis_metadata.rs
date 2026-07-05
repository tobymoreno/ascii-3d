use std::{fs, io, path::Path};

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct CartesianAxesMetadata {
    pub name: String,
    pub version: u32,
    pub units: String,
    pub geometry_asset: String,
    pub origin: OriginMetadata,
    pub axes: Vec<AxisMetadata>,
    pub display: DisplayMetadata,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct OriginMetadata {
    pub position: [f32; 3],
    pub label: String,
    pub group: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AxisMetadata {
    pub id: String,
    pub group_shaft: String,
    pub group_arrow: String,
    pub positive_direction: [f32; 3],
    pub negative_direction: [f32; 3],
    pub length: f32,
    pub positive_endpoint: [f32; 3],
    pub positive_label: String,
    pub negative_label: String,
    pub positive_label_position: [f32; 3],
    pub negative_label_position: [f32; 3],
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct DisplayMetadata {
    pub show_origin: bool,
    pub show_positive_labels: bool,
    pub show_negative_labels: bool,
    pub default_axis_length: f32,
    pub arrowhead_length: f32,
    pub label_strategy: String,
    pub notes: Vec<String>,
}

impl CartesianAxesMetadata {
    pub fn axis(&self, id: &str) -> Option<&AxisMetadata> {
        self.axes.iter().find(|axis| axis.id == id)
    }

    pub fn validate(&self) -> io::Result<()> {
        if self.name.trim().is_empty() {
            return Err(io::Error::other("axis metadata name must not be empty"));
        }

        if self.geometry_asset.trim().is_empty() {
            return Err(io::Error::other(
                "axis metadata geometry_asset must not be empty",
            ));
        }

        if self.axes.is_empty() {
            return Err(io::Error::other(
                "axis metadata must contain at least one axis",
            ));
        }

        for required_id in ["x", "y", "z"] {
            if self.axis(required_id).is_none() {
                return Err(io::Error::other(format!(
                    "axis metadata is missing required '{}' axis",
                    required_id,
                )));
            }
        }

        for axis in &self.axes {
            if axis.id.trim().is_empty() {
                return Err(io::Error::other("axis id must not be empty"));
            }

            if axis.length <= 0.0 {
                return Err(io::Error::other(format!(
                    "axis '{}' length must be greater than zero",
                    axis.id,
                )));
            }

            if axis.positive_label.trim().is_empty() {
                return Err(io::Error::other(format!(
                    "axis '{}' positive label must not be empty",
                    axis.id,
                )));
            }
        }

        Ok(())
    }
}

pub fn load_cartesian_axes_metadata(path: impl AsRef<Path>) -> io::Result<CartesianAxesMetadata> {
    let path = path.as_ref();

    let contents = fs::read_to_string(path).map_err(|error| {
        io::Error::other(format!(
            "failed to read axis metadata {}: {}",
            path.display(),
            error,
        ))
    })?;

    let metadata: CartesianAxesMetadata = serde_json::from_str(&contents).map_err(|error| {
        io::Error::other(format!(
            "failed to parse axis metadata {}: {}",
            path.display(),
            error,
        ))
    })?;

    metadata.validate()?;

    Ok(metadata)
}

#[cfg(test)]
mod tests {
    use super::{CartesianAxesMetadata, load_cartesian_axes_metadata};
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temporary_json_path() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be valid")
            .as_nanos();

        std::env::temp_dir().join(format!("ascii-3d-cartesian-axes-{nanos}.json"))
    }

    fn valid_json() -> &'static str {
        r#"
        {
          "name": "cartesian_axes",
          "version": 1,
          "units": "world",
          "geometry_asset": "cartesian_axes.obj",
          "origin": {
            "position": [0.0, 0.0, 0.0],
            "label": "O",
            "group": "origin_marker"
          },
          "axes": [
            {
              "id": "x",
              "group_shaft": "x_axis_shaft",
              "group_arrow": "x_axis_arrow",
              "positive_direction": [1.0, 0.0, 0.0],
              "negative_direction": [-1.0, 0.0, 0.0],
              "length": 3.0,
              "positive_endpoint": [3.0, 0.0, 0.0],
              "positive_label": "+X",
              "negative_label": "-X",
              "positive_label_position": [3.38, 0.0, 0.0],
              "negative_label_position": [-0.45, 0.0, 0.0]
            },
            {
              "id": "y",
              "group_shaft": "y_axis_shaft",
              "group_arrow": "y_axis_arrow",
              "positive_direction": [0.0, 1.0, 0.0],
              "negative_direction": [0.0, -1.0, 0.0],
              "length": 3.0,
              "positive_endpoint": [0.0, 3.0, 0.0],
              "positive_label": "+Y",
              "negative_label": "-Y",
              "positive_label_position": [0.0, 3.38, 0.0],
              "negative_label_position": [0.0, -0.45, 0.0]
            },
            {
              "id": "z",
              "group_shaft": "z_axis_shaft",
              "group_arrow": "z_axis_arrow",
              "positive_direction": [0.0, 0.0, 1.0],
              "negative_direction": [0.0, 0.0, -1.0],
              "length": 3.0,
              "positive_endpoint": [0.0, 0.0, 3.0],
              "positive_label": "+Z",
              "negative_label": "-Z",
              "positive_label_position": [0.0, 0.0, 3.38],
              "negative_label_position": [0.0, 0.0, -0.45]
            }
          ],
          "display": {
            "show_origin": true,
            "show_positive_labels": true,
            "show_negative_labels": false,
            "default_axis_length": 3.0,
            "arrowhead_length": 0.28,
            "label_strategy": "sidecar_metadata",
            "notes": []
          }
        }
        "#
    }

    #[test]
    fn loads_valid_metadata() {
        let path = temporary_json_path();
        fs::write(&path, valid_json()).expect("test metadata should be written");

        let metadata = load_cartesian_axes_metadata(&path).expect("metadata should load");

        assert_eq!(metadata.name, "cartesian_axes");
        assert_eq!(metadata.axes.len(), 3);
        assert_eq!(
            metadata
                .axis("z")
                .expect("z axis should exist")
                .positive_label,
            "+Z",
        );

        let _ = fs::remove_file(path);
    }

    #[test]
    fn rejects_missing_required_axis() {
        let metadata = CartesianAxesMetadata {
            name: "cartesian_axes".to_string(),
            version: 1,
            units: "world".to_string(),
            geometry_asset: "models/cartesian_axes.obj".to_string(),
            origin: super::OriginMetadata {
                position: [0.0, 0.0, 0.0],
                label: "O".to_string(),
                group: "origin_marker".to_string(),
            },
            axes: Vec::new(),
            display: super::DisplayMetadata {
                show_origin: true,
                show_positive_labels: true,
                show_negative_labels: false,
                default_axis_length: 3.0,
                arrowhead_length: 0.28,
                label_strategy: "sidecar_metadata".to_string(),
                notes: Vec::new(),
            },
        };

        assert!(metadata.validate().is_err());
    }
}
