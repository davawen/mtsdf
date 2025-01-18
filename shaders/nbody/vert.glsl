#version 460

struct Star {
	vec3 position;
	float mass;
	vec3 velocity;
	float _padding;
};

layout (location = 0) in vec3 vertex_pos;

layout (std430, set = 0, binding = 0) readonly buffer Stars {
	Star stars[];
};

void main() {
	Star star = stars[gl_InstanceIndex];
	vec2 pos = 0.05 * vertex_pos.xy * sqrt(star.mass) + star.position.xy;
	gl_Position = vec4(pos / (star.position.z + 1.0), star.position.z + 1.0, 1.0);
}
