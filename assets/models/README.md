# Standard OBJ models

This directory is for standard external geometry files.

Geometry should be stored in normal `.obj` files:

- `v x y z` vertex records
- `f ...` face records
- optional OBJ records may be ignored by ascii-3d until supported

ascii-3d-specific metadata should live outside the OBJ file later, as a sidecar
manifest. The OBJ file should stay portable and tool-friendly.
- `cube.obj`: standard cube mesh used by the rotating OBJ scene.
- `pyramid.obj`: simple reference pyramid mesh.
- `cartesian_axes.obj`: line-based Cartesian axes geometry.
- `quad4.obj`: four-vertex quad mesh used for camera frustum planes.
