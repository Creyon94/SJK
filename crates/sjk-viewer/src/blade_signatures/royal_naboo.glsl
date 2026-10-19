// Royal Naboo: the gilded court of Theed as a blade. A warm white light is held in a cage of
// thin golden bars wound round the blade that glint as they turn; gold dust drifts round it and a
// glint circles the cage. A clash rattles the cage in a gold flash.
// @part Golden cage
// @room 8
// @palette {"corona.rim_cool":[0.6,0.45,0.15],"corona.rim_hot":[1.0,0.85,0.5],"corona.inner":[1.0,0.97,0.9],"core.white":[1,1,0.96],"core.fringe_cool":[1.0,0.8,0.45],"core.fringe_hot":[1.0,0.93,0.75],"light.color":[1.0,0.85,0.55],"trail":[1.0,0.85,0.55]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  return g;
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 gold = vec3(1.0, 0.82, 0.4);
  fx += fxCoils(ro, rd, px, 0.5, L, 1.0, 1.4, 0.1, 4.0, gold)*(1.3 + 0.8*cl);
  fx += fxMotes(ro, rd, px, y0, y1, 1.0, 4.0, 16.0, 0.04, 2.6, gold);
  fx += fxShock(ro, rd, px, L*0.55, 1.0 - cl, 7.0, gold);
  return fx*uFx*part(0.0);
}
