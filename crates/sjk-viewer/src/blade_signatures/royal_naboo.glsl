// Royal Naboo: the grace of Naboo's royal court as a blade. A white-gold blade traced with golden
// filigree that slowly flows up it; three fine golden ribbons curl round it like a queen's
// embroidery, soft petals of gold light drift down past it and a warm royal glow surrounds it. A
// clash sends a gold ring.
// @part Golden filigree
// @part Petals of light
// @part Royal glow
// @room 8
// @palette {"corona.rim_cool":[0.6,0.45,0.15],"corona.rim_hot":[1.0,0.85,0.5],"corona.inner":[1.0,0.97,0.9],"core.white":[1,1,0.96],"core.fringe_cool":[1.0,0.8,0.45],"core.fringe_hot":[1.0,0.93,0.75],"light.color":[1.0,0.85,0.55],"trail":[1.0,0.85,0.55]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float fil = halo(abs(sin(b.y*0.9 - uTime*0.6 + 3.0*sin(b.x*6.0))) - 0.0, 0.12)*part(0.0);
  return recol(g, vec3(1.0, 0.95, 0.82)) + lvl(g)*vec3(1.0, 0.75, 0.3)*fil*0.8;
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 gold = vec3(1.0, 0.78, 0.35), rose = vec3(1.0, 0.7, 0.55);
  fx += fxStreams(ro, rd, px, y0, y1, 3.0, 2.5, 0.3, 1.2, 5.5, gold)*part(0.0);
  fx += fxFlakes(ro, rd, px, y0, y1, 3.5, 12.0, 2.2, 0.4, 0.5, 0.8, 0.8, 4.7, rose)*part(1.0);
  fx += fxAura(ro, rd, y0, y1, 2.4, gold)*0.4*part(2.0);
  fx += fxFlakes(ro, rd, px, y0, y1, 4.5, 10.0, 1.6, 0.45, 0.5, 0.9, 1.0, 9.7, vec3(1.0, 0.75, 0.6))*part(1.0);
  fx += fxMotes(ro, rd, px, y0, y1, 0.8, 3.5, 22.0, 0.03, 6.8, vec3(1.0, 0.9, 0.6))*part(2.0);
  return fx*uFx;
}
