// Thermal Fuse: a thermal detonator's burning fuse, as a blade. A white-hot point burns slowly down
// the blade toward the hilt, spraying sparks as it goes, leaving the blade a little dimmer behind
// it, then starts again from the top.
// @part Burning fuse
// @part Fuse sparks
// @room 8
// @palette {"corona.rim_cool":[0.5,0.3,0.05],"corona.rim_hot":[1.0,0.8,0.4],"corona.inner":[1.0,0.95,0.8],"core.white":[1,0.98,0.92],"core.fringe_cool":[1.0,0.75,0.3],"core.fringe_hot":[1.0,0.92,0.7],"light.color":[1.0,0.8,0.4],"trail":[1.0,0.8,0.4]}

vec3 thermal_fuse_glow0(vec3 g, vec2 b, float r, float px){
  float yf = uLength*(0.95 - 0.8*fract(uTime/7.0));
  float hot = halo(b.y - yf, 0.8);
  float burnt = smoothstep(yf, yf + 2.0, b.y);
  return g*(1.0 - 0.3*burnt*part(0.0)) + lvl(g)*vec3(1.0, 0.95, 0.85)*hot*1.5*part(0.0);
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  vec3 o = thermal_fuse_glow0(g, b, r, px);
  return max(g + (o - g)*2.40, vec3(0.0));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  float yf = L*(0.95 - 0.8*fract(t/7.0));
  fx += fxSparks(ro, rd, px, yf - 0.4, yf + 0.4, 16.0, 2.6, 2.0, 1.9, vec3(1.0, 0.75, 0.35))*part(1.0);
  return fx*uFx*2.40;
}
