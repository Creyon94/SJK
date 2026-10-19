// Dathomir Blood Moon: the Nightsisters' red night as a blade. A dark crimson blade with a
// shadowed heart, like Sol's Void in blood red; drops of red light drip off it and fall, a slow red
// shimmer climbs it, and shadow copies trail every swing. A clash flashes a crimson ring.
// @part Blood drips
// @room 8
// @palette {"corona.rim_cool":[0.35,0.0,0.02],"corona.rim_hot":[1.0,0.12,0.08],"corona.inner":[0.25,0.0,0.02],"core.white":[0.55,0.05,0.05],"core.fringe_cool":[0.6,0.02,0.02],"core.fringe_hot":[1.0,0.2,0.15],"light.color":[0.9,0.08,0.06],"trail":[0.9,0.08,0.06]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  return g*(0.9 + 0.3*bands(b.y, 0.06, 0.4));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 red = vec3(1.0, 0.08, 0.05);
  fx += fxAura(ro, rd, y0, y1, 2.6, red)*0.8;
  fx += fxMotes(ro, rd, px, y0, y1, 1.0, 4.5, 18.0, -0.06, 4.4, red);
  fx += fxFall(ro, rd, px, y0, y1, 1.2, 10.0, 5.0, 0.6, 0.0, 2.9, red);
  fx += fxShock(ro, rd, px, L*0.5, 1.0 - cl, 7.0, red);
  return fx*uFx*part(0.0);
}
