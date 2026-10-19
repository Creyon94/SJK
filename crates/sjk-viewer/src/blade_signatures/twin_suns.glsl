// Twin Suns: Tatooine's two suns at full blaze. A blade of gold and orange halves rolling into each
// other, loops of solar plasma arching off it and snapping back, the air shimmering with heat and a
// glint at the tip. A swing throws more loops; a clash a gold ring.
// @part Sun glow
// @arcs Solar prominences
// @room 8
// @palette {"corona.rim_cool":[0.6,0.2,0.02],"corona.rim_hot":[1.0,0.6,0.15],"corona.inner":[1.0,0.9,0.6],"core.white":[1,0.97,0.85],"core.fringe_cool":[1.0,0.5,0.1],"core.fringe_hot":[1.0,0.85,0.5],"light.color":[1.0,0.62,0.2],"trail":[1.0,0.62,0.2]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  vec3 a = vec3(1.0, 0.85, 0.4), o = vec3(1.0, 0.45, 0.1);
  return recol(g, mix(a, o, bands(b.y, 0.05, 0.3)))*1.1;
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 gold = vec3(1.0, 0.7, 0.25);
  fx += fxAura(ro, rd, y0, y1, 2.6, gold)*0.7;
  fx += fxShock(ro, rd, px, L*0.55, 1.0 - cl, 8.0, gold);
  return fx*uFx*part(0.0);
}
