// Unknown Regions: the uncharted dark beyond the galaxy's edge. A deep violet blade with a shadowed
// heart like Sol's Void, nebula clouds swirling slowly inside it, and faint stars drifting round it
// and into it. A clash flashes a violet ring.
// @part Drifting stars
// @room 8
// @palette {"corona.rim_cool":[0.15,0.05,0.4],"corona.rim_hot":[0.6,0.3,1.0],"corona.inner":[0.12,0.04,0.3],"core.white":[0.45,0.3,0.8],"core.fringe_cool":[0.35,0.15,0.9],"core.fringe_hot":[0.7,0.5,1.0],"light.color":[0.5,0.3,1.0],"trail":[0.5,0.3,1.0]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float n = fbm3(vec2(b.x*1.2 + uTime*0.1, b.y*0.25 - uTime*0.15));
  return g*(0.7 + 0.7*n);
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 vio = vec3(0.6, 0.35, 1.0), white = vec3(0.85, 0.85, 1.0);
  fx += fxMotes(ro, rd, px, y0, y1, 1.2, 5.0, 26.0, 0.03, 6.2, white);
  fx += fxAura(ro, rd, y0, y1, 2.8, vio)*0.7;
  fx += fxShock(ro, rd, px, L*0.55, 1.0 - cl, 8.0, vio);
  return fx*uFx*part(0.0);
}
