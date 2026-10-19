// Takodana Rain: soft rain on the lake below Takodana's castle, as a blade. Drops land on the blade
// and send ripples of light running along it both ways, and a light rain falls past it.
// @part Ripples
// @part Rain
// @room 8
// @palette {"corona.rim_cool":[0.15,0.3,0.5],"corona.rim_hot":[0.55,0.8,1.0],"corona.inner":[0.88,0.95,1.0],"core.white":[0.95,0.98,1.0],"core.fringe_cool":[0.45,0.7,1.0],"core.fringe_hot":[0.85,0.93,1.0],"light.color":[0.55,0.8,1.0],"trail":[0.58,0.82,1.0]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float rip = 0.0;
  for (int i = 0; i < 3; i++){
    float fi = float(i);
    float cyc = uTime*0.5 + fi*0.33;
    float k = floor(cyc), age = fract(cyc)*2.0;
    float yd = uLength*(0.25 + 0.65*h11(k*2.3 + fi));
    float d = abs(b.y - yd);
    rip += max(sin(d*3.0 - age*8.0), 0.0)*exp(-d*0.35)*exp(-age*1.5);
  }
  return g*(0.85 + 0.6*rip*part(0.0));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  fx += fxFall(ro, rd, px, y0, y1, 3.0, 12.0, 16.0, 0.8, 0.0, 5.7, vec3(0.75, 0.85, 1.0))*0.6*part(1.0);
  return fx*uFx;
}
