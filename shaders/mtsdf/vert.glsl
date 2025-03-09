#version 460

layout (location = 0) in vec3 vertex_pos;

layout (location = 0) out vec3 window_pos;

layout (set = 1, binding = 0) uniform Transform {
	mat4 model;
	mat4 view;
} transform;

void main() {
	vec4 pos = transform.model * vec4(vertex_pos, 1.0);
	window_pos = pos.xyz;
	gl_Position = transform.view * pos;
}
