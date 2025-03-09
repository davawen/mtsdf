#version 450

layout (location = 0) out vec4 frag_color;

layout (location = 0) in vec3 pos;

layout (set = 3, binding = 0) uniform Circle {
	float radius;
	vec2 center;
} circle;

void main() {
	vec2 diff = pos.xy - circle.center;
	if (diff.x*diff.x + diff.y*diff.y > circle.radius*circle.radius) discard;
	
	frag_color = vec4(1.0, 1.0, 0.0, 1.0);
}
