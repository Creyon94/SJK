// Temple Sunbeam: sunlight falling through a high Jedi Temple window, as a blade. A warm shaft of
// light round the blade, gently brighter in slow waves, with fine dust drifting slowly through it;
// nothing reaches past the tip. A clash flares it.
// @part Dust in the light
// @room 8
// @palette {"corona.rim_cool":[0.6,0.4,0.12],"corona.rim_hot":[1.0,0.85,0.5],"corona.inner":[1.0,0.96,0.82],"core.white":[1.0,0.99,0.93],"core.fringe_cool":[1.0,0.82,0.45],"core.fringe_hot":[1.0,0.95,0.8],"light.color":[1.0,0.85,0.5],"trail":[1.0,0.84,0.5]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  return g*(1.0 + 0.12*bands(b.y, 0.03, 0.2));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 warm = vec3(1.0, 0.88, 0.6);
  float yt = L*0.86;
  fx += fxAura(ro, rd, y0, yt, 3.0, warm)*0.45;
  fx += fxMotes(ro, rd, px, y0, yt - 2.0, 0.6, 3.0, 26.0, 0.012, 2.9, warm)*0.6;
  return fx*uFx*part(0.0);
}
