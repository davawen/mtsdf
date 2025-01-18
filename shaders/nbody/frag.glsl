#version 460

layout (location = 0) out vec4 fragColor;

layout (set = 3, binding = 0) uniform Uniform {
	float time;
};

void main() {
	fragColor = vec4(1.0, sin(time)*0.5 + 0.5, sin(time*0.7 + 1.0)*0.25 + 0.75, 0.5);
}
