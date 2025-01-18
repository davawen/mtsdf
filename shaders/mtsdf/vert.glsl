#version 460

layout (location = 0) in vec3 vertex_pos;

// layout (binding = 0) uniform Camera {
//
// }

void main() {
	gl_Position = vec4(vertex_pos, 1.0);
}
