// Unknown Regions: the uncharted dark beyond the galaxy's edge. A deep violet blade with a shadowed
// heart, nebula swirling slowly inside it and round it in soft violet clouds, faint stars drifting
// through them and thin threads of starlight winding up it. A clash flares it.
// @part Nebula
// @part Drifting stars
// @part Starlight
// @room 8
// @palette {"corona.rim_cool":[0.15,0.05,0.4],"corona.rim_hot":[0.6,0.3,1.0],"corona.inner":[0.12,0.04,0.3],"core.white":[0.45,0.3,0.8],"core.fringe_cool":[0.35,0.15,0.9],"core.fringe_hot":[0.7,0.5,1.0],"light.color":[0.5,0.3,1.0],"trail":[0.5,0.3,1.0]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float n = fbm3(vec2(b.x*1.2 + uTime*0.1, b.y*0.25 - uTime*0.15));
  float heart = 1.0 - 0.4*exp(-pow(b.x/0.25, 2.0));
  return g*heart*(0.6 + 0.8*n);
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 v = vec3(0.5, 0.3, 1.0), w = vec3(0.85, 0.85, 1.0);
  fx += fxSmoke(ro, rd, y0, y1, 4, 0.6, 8.8, v)*1.1*part(0.0);
  fx += fxMotes(ro, rd, px, y0, y1, 0.8, 4.5, 28.0, 0.015, 3.9, w)*0.8*part(1.0);
  fx += fxStreams(ro, rd, px, y0, y1, 2.0, 2.4, 0.25, 1.0, 4.4, v)*0.6*part(2.0);
  return fx*uFx;
}
