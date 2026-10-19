// Assassin Droid: an assassin droid's weapon as a blade. Its light is cut into hard plated bands
// that shift like machinery; a red scanner ring sweeps up and down it, sparks spit off its joints
// and a thin grey smoke rises off it. A clash flares the bands.
// @part Scanner
// @part Sparks
// @part Droid smoke
// @room 8
// @palette {"corona.rim_cool":[0.5,0.05,0.02],"corona.rim_hot":[1.0,0.3,0.1],"corona.inner":[1.0,0.85,0.7],"core.white":[1,0.95,0.9],"core.fringe_cool":[1.0,0.3,0.1],"core.fringe_hot":[1.0,0.75,0.55],"light.color":[1.0,0.35,0.12],"trail":[1.0,0.35,0.12]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float seg = fract(b.y/2.5 + floor(uTime*0.8 + h11(floor(b.y/2.5)))*0.0);
  float joint = halo((0.5 - abs(seg - 0.5))*2.5, 0.06);
  float scan = halo(b.y - (0.5 + 0.5*sin(uTime*0.9))*uLength, 0.5)*part(0.0);
  return g*(0.85 + 0.5*scan + 0.6*clamp(uClash, 0.0, 1.0)) + lvl(g)*vec3(1.0, 0.85, 0.7)*joint*0.6;
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 c = vec3(1.0, 0.75, 0.5), red = vec3(1.0, 0.15, 0.1);
  float ys = mix(y0, y1, 0.5 + 0.5*sin(t*0.9));
  float d = ringD(ro, rd, ys, 2.2);
  fx += (vec3(1.0, 0.6, 0.55)*halo(d, max(0.04, px))*1.1 + red*halo(d, 0.4)*0.8)*part(0.0);
  fx += fxSparks(ro, rd, px, y0, y1, 12.0 + 10.0*sw, 3.5, 1.4, 2.7, c)*part(1.0);
  fx += fxSmoke(ro, rd, y0, y1, 2.6, 2.5, 4.2, vec3(0.45, 0.45, 0.5))*0.8*part(2.0);
  return fx*uFx;
}
