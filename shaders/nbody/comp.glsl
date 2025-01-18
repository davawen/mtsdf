#version 460

layout (local_size_x = 32) in;

struct Star {
	vec3 position;
	float mass;
	vec3 velocity;
	float _padding;
};

layout (std430, set = 0, binding = 0) readonly buffer InputStars { Star in_stars[]; };
layout (std430, set = 1, binding = 0) writeonly buffer OutputStars { Star out_stars[]; };
layout (set = 2, binding = 0) uniform Uniform {
	uint num_stars;
};

const float G = 0.01;
const float dt = 0.01;

void main() {
	uint index = gl_GlobalInvocationID.x;
	if (index >= num_stars) return;

	Star star = in_stars[index];
	vec3 force = vec3(0.0);
	for (uint i = 0; i < num_stars; i++) {
		if (i == index) continue;

		Star other = in_stars[i];

		vec3 diff = other.position - star.position;
		float len = length(diff);

		if (len <= 0.001) continue;

		float len_cubed = len*len*len;
		vec3 f = other.mass * diff / len;

		force += f;
	}

	star.velocity += G*dt*force;
	star.position += star.velocity*dt;
	out_stars[index] = star;
}
