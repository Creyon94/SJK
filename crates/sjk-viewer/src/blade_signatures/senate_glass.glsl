// Senate Glass: the tall coloured windows of the old Galactic Senate, as a blade. Tall panes of deep
// red, blue and gold set in dark leading, with light slowly moving through them as if the sun
// were passing outside.
// @part Light through the glass
// @motes Dust in the light
// @room 8
// @palette {"corona.rim_cool":[0.45,0.2,0.4],"corona.rim_hot":[1.0,0.7,0.45],"corona.inner":[1.0,0.92,0.8],"core.white":[1.0,0.97,0.9],"core.fringe_cool":[0.8,0.35,0.5],"core.fringe_hot":[1.0,0.85,0.7],"light.color":[1.0,0.55,0.45],"trail":[0.95,0.5,0.55]}

vec3 senate_glass_glow0(vec3 g, vec2 b, float r, float px){
  vec2 q = vec2(b.x*2.2 + 0.5, b.y/5.0);
  vec2 id = floor(q), f = fract(q);
  float h = h21(id);
  vec3 pane = h < 0.33 ? vec3(0.95, 0.2, 0.2) : (h < 0.66 ? vec3(0.25, 0.45, 1.0) : vec3(1.0, 0.8, 0.3));
  float lead = smoothstep(0.0, 0.08, min(min(f.x, 1.0 - f.x), min(f.y, 1.0 - f.y)*2.5));
  float sun = 0.75 + 0.35*sin(b.y*0.12 - uTime*0.4);
  return recol(g, pane)*(0.45 + 0.55*lead)*mix(1.0, sun, part(0.0));
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  vec3 o = senate_glass_glow0(g, b, r, px);
  return max(g + (o - g)*2.40, vec3(0.0));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  fx += fxAura(ro, rd, y0, y1, 2.0, vec3(0.9, 0.6, 0.6))*0.25*part(0.0);
  return fx*uFx*2.40;
}
