// Malachor Forge: a blade still on the Sith anvils of Malachor. Red-hot steel glows white where
// each hammer blow lands, sparks fly off on every blow and embers drip off it; the air shimmers.
// A swing fans the heat; a clash is a great blow.
// @part Hammer sparks
// @room 8
// @palette {"corona.rim_cool":[0.5,0.08,0.02],"corona.rim_hot":[1.0,0.4,0.1],"corona.inner":[1.0,0.8,0.6],"core.white":[1,0.95,0.85],"core.fringe_cool":[1.0,0.35,0.08],"core.fringe_hot":[1.0,0.7,0.45],"light.color":[1.0,0.42,0.12],"trail":[1.0,0.42,0.12]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float n = floor(uTime/1.1), ph = fract(uTime/1.1)*1.1;
  float spot = exp(-pow((b.y - uLength*(0.25 + 0.6*h11(n*3.7)))/1.5, 2.0))*exp(-ph*2.0);
  return g*(0.9 + 1.4*spot + 0.3*clamp(uSwing, 0.0, 1.0));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 hot = vec3(1.0, 0.55, 0.15);
  float n = floor(t/1.1), ph = fract(t/1.1);
  float yb = L*(0.25 + 0.6*h11(n*3.7));
  fx += fxSparks(ro, rd, px, yb - 1.0, yb + 1.0, 14.0, 5.0, 0.9, n, hot)*(1.0 - ph);
  fx += fxShock(ro, rd, px, L*0.55, 1.0 - cl, 8.0, hot);
  return fx*uFx*part(0.0);
}
