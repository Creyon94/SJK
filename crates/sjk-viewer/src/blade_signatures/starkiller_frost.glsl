// Starkiller Frost: a bolt of lightning frozen in the ice of Starkiller Base. Inside a frosted
// blade a jagged bolt is locked in place, glowing; frost mist rolls off it. A swing makes the bolt
// flicker; a clash cracks it free for an instant, real lightning leaping off the blade.
// @part Breaking free
// @room 8
// @palette {"corona.rim_cool":[0.3,0.5,0.85],"corona.rim_hot":[0.75,0.9,1.0],"corona.inner":[0.92,0.97,1.0],"core.white":[1,1,1],"core.fringe_cool":[0.55,0.75,1.0],"core.fringe_hot":[0.88,0.96,1.0],"light.color":[0.65,0.85,1.0],"trail":[0.65,0.85,1.0]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float d = boltD(vec2(b.x*3.0, b.y), vec2(0.0, 0.0), vec2(0.0, uLength), 7.0, 0.04);
  float fl = 0.8 + 0.2*sin(uTime*30.0)*clamp(uSwing, 0.0, 1.0);
  return g*(0.85 + 1.2*halo(d, 0.25)*fl);
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 ice = vec3(0.7, 0.88, 1.0);
  fx += fxArcs(ro, rd, px, y0, y1, 5.0, 4.0, 3.0, 2.2, ice)*cl;
  fx += fxMotes(ro, rd, px, y0, y1, 1.0, 3.5, 12.0, -0.05, 8.3, vec3(0.9, 0.97, 1.0));
  fx += fxShock(ro, rd, px, L*0.55, 1.0 - cl, 8.0, ice);
  return fx*uFx*part(0.0);
}
