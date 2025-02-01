#version 460

layout (location = 0) in vec3 vertex_pos;

layout (set = 1, binding = 0) uniform Position {
	mat4 transform;
} pos;

void main() {
	gl_Position = pos.transform * vec4(vertex_pos, 1.0);
}
