// Force Echo: a blade that echoes through time, as the Force remembers. Afterimages linger where
// it has been and an echo of it sways beside it; rings of time tick up it once a second, two thin
// streams of time wind slowly round it and small lights of remembered moments drift up. A clash flares it.
// @part Rings of time
// @part Time streams
// @part Echo lights
// @room 8
// @palette {"corona.rim_cool":[0.25,0.35,0.75],"corona.rim_hot":[0.6,0.75,1.0],"corona.inner":[0.85,0.92,1.0],"core.white":[1,1,1],"core.fringe_cool":[0.5,0.65,1.0],"core.fringe_hot":[0.85,0.93,1.0],"light.color":[0.6,0.75,1.0],"trail":[0.6,0.75,1.0]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float tick = halo(fract(uTime) - 0.0, 0.08);
  float step_ = halo(abs(fract(b.y/6.0 - uTime/6.0) - 0.5), 0.03);
  return g*(0.85 + 0.25*tick + 0.5*step_*part(0.0));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 c = vec3(0.65, 0.8, 1.0);
  fx += fxRingWaves(ro, rd, px, y0, y1, 3.0, 1.6, 6.0, 0.1, c)*part(0.0);
  fx += fxStreams(ro, rd, px, y0, y1, 2.0, 2.8, 0.12, 0.8, 2.3, c)*0.8*part(1.0);
  fx += fxMotes(ro, rd, px, y0, y1, 1.0, 4.0, 16.0, 0.05, 7.4, c)*part(2.0);
  return fx*uFx;
}
