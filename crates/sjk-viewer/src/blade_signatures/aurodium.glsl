// Aurodium: the rarest gold in the galaxy as a blade. Molten ripples run up a gold blade, drops
// of gold drip off it, and three glints circle it slowly, as Sol's glints do. A swing makes the
// ripples race; a clash a gold ring.
// @part Gold dust
// @room 8
// @palette {"corona.rim_cool":[0.6,0.4,0.05],"corona.rim_hot":[1.0,0.82,0.3],"corona.inner":[1.0,0.95,0.7],"core.white":[1,0.97,0.85],"core.fringe_cool":[1.0,0.75,0.25],"core.fringe_hot":[1.0,0.9,0.6],"light.color":[1.0,0.82,0.35],"trail":[1.0,0.82,0.35]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  return g*(0.85 + 0.35*bands(b.y, 0.25, 0.8 + 1.5*clamp(uSwing, 0.0, 1.0)));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 gold = vec3(1.0, 0.82, 0.35);
  fx += fxAura(ro, rd, y0, y1, 2.4, gold)*0.6;
  fx += fxMotes(ro, rd, px, y0, y1, 1.0, 4.0, 16.0, -0.05, 5.8, gold);
  fx += fxShock(ro, rd, px, L*0.55, 1.0 - cl, 7.0, gold);
  return fx*uFx*part(0.0);
}
