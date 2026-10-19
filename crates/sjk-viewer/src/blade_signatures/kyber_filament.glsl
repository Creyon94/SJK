// Kyber Filament: a fine kyber filament sealed in amber glass, as a blade. A zigzag of white light
// glows inside a warm amber blade and flickers now and then.
// @part Flicker
// @room 8
// @palette {"corona.rim_cool":[0.45,0.2,0.03],"corona.rim_hot":[1.0,0.6,0.18],"corona.inner":[1.0,0.85,0.55],"core.white":[1.0,0.92,0.75],"core.fringe_cool":[1.0,0.55,0.12],"core.fringe_hot":[1.0,0.8,0.45],"light.color":[1.0,0.62,0.22],"trail":[1.0,0.6,0.2]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float z = abs(fract(b.y*0.6) - 0.5)*0.6 - 0.15;
  float fil = halo(abs(b.x - z), 0.04);
  float fl = 1.0 - 0.25*step(0.92, h11(floor(uTime*10.0)))*part(0.0);
  return recol(g, vec3(1.0, 0.7, 0.3))*0.8*fl + lvl(g)*vec3(1.0, 0.95, 0.85)*fil*fl;
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  fx += vec3(0.0);
  return fx*uFx;
}
