// Hoth Avalanche: the snowfields of Hoth as a blade. An ice-blue blade with frost bands drifting
// down it; snow pours down round it in a steady rush and cold mist rolls off it, and every few
// seconds a white surge races down to the hilt. A swing throws a burst of snow; a clash a cold ring.
// @part Snowfall
// @room 8
// @palette {"corona.rim_cool":[0.35,0.55,0.85],"corona.rim_hot":[0.75,0.9,1.0],"corona.inner":[0.92,0.97,1.0],"core.white":[1,1,1],"core.fringe_cool":[0.6,0.8,1.0],"core.fringe_hot":[0.9,0.97,1.0],"light.color":[0.7,0.85,1.0],"trail":[0.7,0.85,1.0]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float surge = halo(b.y - (uLength + 4.0 - mod(uTime*18.0, uLength + 14.0)), 2.0);
  return g*(0.85 + 0.25*bands(b.y, 0.12, -1.2) + 0.9*surge);
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 snow = vec3(0.9, 0.96, 1.0);
  fx += fxFall(ro, rd, px, y0, y1, 3.5, 22.0 + 10.0*sw, 9.0, 0.5, 0.1, 1.3, snow);
  fx += fxMotes(ro, rd, px, y0, y1, 1.0, 4.0, 14.0, -0.08, 2.1, snow);
  fx += fxShock(ro, rd, px, L*0.5, 1.0 - cl, 7.0, snow);
  return fx*uFx*part(0.0);
}
