// Force Echo: a blade that echoes through time, as the Force remembers. Afterimages linger
// where it has been, a faint echo of it sways beside it, and soft rings of time tick slowly up
// it, one a second. A clash rings every echo at once.
// @part Rings of time
// @room 8
// @palette {"corona.rim_cool":[0.25,0.35,0.75],"corona.rim_hot":[0.6,0.75,1.0],"corona.inner":[0.85,0.92,1.0],"core.white":[1,1,1],"core.fringe_cool":[0.5,0.65,1.0],"core.fringe_hot":[0.85,0.93,1.0],"light.color":[0.6,0.75,1.0],"trail":[0.6,0.75,1.0]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float tick = halo(fract(uTime) - 0.0, 0.08);
  return g*(0.9 + 0.2*tick);
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 c = vec3(0.65, 0.8, 1.0);
  float q = fract(t);
  fx += fxShock(ro, rd, px, mix(y0, y1, fract(floor(t)*0.37)), q, 3.0, c)*0.9;
  fx += fxMotes(ro, rd, px, y0, y1, 1.0, 4.0, 16.0, 0.05, 7.4, c);
  fx += fxShock(ro, rd, px, L*0.55, 1.0 - cl, 8.0, c);
  return fx*uFx*part(0.0);
}
