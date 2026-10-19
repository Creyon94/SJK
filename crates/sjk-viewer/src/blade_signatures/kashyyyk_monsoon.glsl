// Kashyyyk Monsoon: the great rains over the Wookiee forests. Sheets of rain lash down past a green
// blade, mist rising off it, and now and then a flash of lightning lights everything. A swing
// drives the rain; a clash a flash and a ring of spray.
// @part Rain and lightning
// @room 8
// @palette {"corona.rim_cool":[0.1,0.4,0.15],"corona.rim_hot":[0.45,0.95,0.5],"corona.inner":[0.88,1.0,0.9],"core.white":[1,1,1],"core.fringe_cool":[0.35,0.85,0.45],"core.fringe_hot":[0.8,1.0,0.85],"light.color":[0.45,0.95,0.55],"trail":[0.45,0.95,0.55]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float fl = step(0.93, h11(floor(uTime*3.0)));
  return g*(0.9 + 0.8*fl);
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 rain = vec3(0.75, 0.9, 1.0);
  fx += fxFall(ro, rd, px, y0, y1, 4.0, 26.0 + 6.0*sw, 22.0, 1.3, 0.25, 1.9, rain);
  fx += fxArcs(ro, rd, px, y0, y1, 1.0, 4.0, 0.6, 9.9, vec3(0.8, 0.9, 1.0));
  fx += fxShock(ro, rd, px, L*0.55, 1.0 - cl, 8.0, rain);
  return fx*uFx*part(0.0);
}
