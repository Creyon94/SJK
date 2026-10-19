// Hypermatter: a capital ship's hypermatter stream as a blade. Bright pulses race up the blade in
// pairs, chasing each other, and meet in a flash just below the tip that throws a few sparks; a
// glint turns at the tip. A swing sends more pulses; a clash a bright ring.
// @part Hypermatter flash
// @room 8
// @palette {"corona.rim_cool":[0.55,0.35,0.05],"corona.rim_hot":[1.0,0.8,0.3],"corona.inner":[1.0,0.95,0.75],"core.white":[1,1,0.95],"core.fringe_cool":[1.0,0.7,0.2],"core.fringe_hot":[1.0,0.9,0.6],"light.color":[1.0,0.8,0.35],"trail":[1.0,0.8,0.35]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float p = pow(fract(b.y/10.0 - uTime*1.2), 12.0) + pow(fract(b.y/10.0 - uTime*1.2 + 0.5), 12.0);
  return g*(0.85 + 1.2*p*step(0.0, b.y));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 gold = vec3(1.0, 0.8, 0.35);
  float q = fract(t*0.6);
  fx += fxFlare(ro, rd, vec3(0.0, L - 3.0, 0.0), px, 0.2, 3.0, 4.0, 0.4, gold)*exp(-q*6.0)*1.5;
  fx += fxSparks(ro, rd, px, L - 5.0, L - 1.0, 8.0 + 8.0*sw, 4.0, 0.6, 2.5, gold);
  fx += fxShock(ro, rd, px, L*0.55, 1.0 - cl, 8.0, gold);
  return fx*uFx*part(0.0);
}
