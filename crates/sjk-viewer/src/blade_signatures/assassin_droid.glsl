// Assassin Droid: the burning core of an IG assassin droid as a blade. Dark armour plates sit along
// the blade and slide apart in turn to show its hot core; a targeting line sweeps up and down it.
// A swing opens every plate; a clash flashes the core.
// @part Sparks
// @room 8
// @solid 1
// @palette {"corona.rim_cool":[0.5,0.05,0.02],"corona.rim_hot":[1.0,0.3,0.1],"corona.inner":[1.0,0.85,0.7],"core.white":[1,0.95,0.9],"core.fringe_cool":[1.0,0.3,0.1],"core.fringe_hot":[1.0,0.75,0.55],"light.color":[1.0,0.35,0.12],"trail":[1.0,0.35,0.12]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float scan = halo(b.y - (0.5 + 0.5*sin(uTime*0.9))*uLength, 0.4);
  return g*(0.95 + 0.6*scan + 0.6*clamp(uClash, 0.0, 1.0));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
vec4 sig_solid(vec4 o, vec2 b, float px){
  if (uFx < 0.01 || b.y < 0.5 || b.y > uLength - 0.5) return o;
  float k = floor(b.y/3.0), f = fract(b.y/3.0);
  float open = smoothstep(0.3, 0.7, sin(uTime*1.3 + k*1.7)*0.5 + 0.5 + clamp(uSwing, 0.0, 1.0));
  float w = 0.75 - 0.5*open;
  float gap = smoothstep(0.0, 0.08, f)*smoothstep(1.0, 0.92, f);
  float cov = fillD(abs(abs(b.x) - w) - 0.18, px)*gap*(1.0 - open*0.85);
  vec3 plate = vec3(0.12, 0.12, 0.14)*(0.7 + 0.3*sign(b.x)) + vec3(0.6, 0.15, 0.05)*halo(abs(b.x) - w + 0.18, 0.05);
  return vec4(mix(o.rgb, plate, cov), max(o.a, cov));
}
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 red = vec3(1.0, 0.3, 0.1);
  fx += fxSparks(ro, rd, px, y0, y1, 4.0 + 10.0*sw, 3.5, 0.7, 4.1, red);
  fx += fxShock(ro, rd, px, L*0.55, 1.0 - cl, 7.0, red);
  return fx*uFx*part(0.0);
}
