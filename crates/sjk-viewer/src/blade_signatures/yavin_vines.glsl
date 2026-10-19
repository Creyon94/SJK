// Yavin Vines: the jungle round the temples of Yavin 4. Vines of green light wind up the blade,
// buds of light swelling along them, and pollen of light drifts up off it in the warm air. A swing
// shakes the pollen loose; a clash a ring of leaves of light.
// @part Vines and pollen
// @room 8
// @palette {"corona.rim_cool":[0.15,0.4,0.05],"corona.rim_hot":[0.55,0.95,0.3],"corona.inner":[0.9,1.0,0.85],"core.white":[1,1,0.95],"core.fringe_cool":[0.45,0.85,0.25],"core.fringe_hot":[0.85,1.0,0.75],"light.color":[0.55,0.95,0.35],"trail":[0.55,0.95,0.35]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  return g;
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 leaf = vec3(0.55, 1.0, 0.35), pollen = vec3(1.0, 0.95, 0.6);
  fx += fxCoils(ro, rd, px, y0, y1, 1.0, 0.7, 0.15, 2.0, leaf);
  fx += fxMotes(ro, rd, px, y0, y1, 1.0, 4.5, 16.0 + 10.0*sw, 0.05, 1.8, pollen);
  fx += fxShock(ro, rd, px, L*0.55, 1.0 - cl, 7.0, leaf);
  return fx*uFx*part(0.0);
}
