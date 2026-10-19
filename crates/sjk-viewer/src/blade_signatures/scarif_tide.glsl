// Scarif Tide: the turquoise shallows of Scarif as a blade. Two currents spiral up a sea-green
// blade, slow waves of light roll up it and bubbles rise round it, glinting. A swing churns the
// currents; a clash sends a ring of spray.
// @part Currents
// @room 8
// @palette {"corona.rim_cool":[0.0,0.4,0.45],"corona.rim_hot":[0.3,0.95,0.9],"corona.inner":[0.85,1.0,0.98],"core.white":[1,1,1],"core.fringe_cool":[0.2,0.85,0.85],"core.fringe_hot":[0.75,1.0,0.97],"light.color":[0.3,0.9,0.85],"trail":[0.3,0.9,0.85]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  return g*(0.85 + 0.35*bands(b.y, 0.08, 1.0));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 sea = vec3(0.3, 0.95, 0.9), foam = vec3(0.85, 1.0, 1.0);
  fx += fxCoils(ro, rd, px, y0, y1, 1.6, 0.8, 0.5, 2.0, sea);
  fx += fxMotes(ro, rd, px, y0, y1, 0.8, 3.5, 20.0, 0.1, 3.2, foam);
  fx += fxShock(ro, rd, px, L*0.55, 1.0 - cl, 8.0, foam);
  return fx*uFx*part(0.0);
}
