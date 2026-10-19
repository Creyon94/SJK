// Dagobah Wisps: the swamp lights of Dagobah as a blade. A few warm wisps of light circle the blade
// lazily, bobbing up and down, and scatter wide when it swings; a faint swamp mist rises off it.
// A clash flares it.
// @part Wisps
// @room 8
// @palette {"corona.rim_cool":[0.55,0.36,0.12],"corona.rim_hot":[1.0,0.78,0.42],"corona.inner":[1.0,0.92,0.74],"core.white":[1.0,0.97,0.88],"core.fringe_cool":[1.0,0.76,0.4],"core.fringe_hot":[1.0,0.93,0.76],"light.color":[1.0,0.78,0.45],"trail":[1.0,0.76,0.42]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  return g;
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 warm = vec3(1.0, 0.85, 0.5);
  return fx*uFx*part(0.0);
}
// only what circles the blade is drawn in 3D
vec3 sig_orbit(vec3 ro, vec3 rd, float px){
  if (uFx < 0.01 || part(0.0) < 0.5) return vec3(0.0);
  vec3 acc = vec3(0.0);
  vec3 c = vec3(1.0, 0.85, 0.5);
  float L = uLength, sw = clamp(uSwing, 0.0, 1.0), ta;
  for (int i = 0; i < 6; i++){
    float fi = float(i);
    float y = L*(0.35 + 0.1*fi) + 0.8*sin(uTime*0.7 + fi*1.9);
    float r = 1.4 + 0.6*sin(uTime*0.5 + fi) + 2.5*sw;
    vec3 p = around(r, uTime*(0.12 + 0.05*h11(fi*3.3)) + h11(fi*7.1), y);
    float d = rayPt(ro, rd, p, ta);
    float tw = 0.7 + 0.3*sin(uTime*4.0 + fi*2.7);
    acc += (vec3(1.0)*halo(d, max(0.07, px))*1.4 + c*halo(d, 0.35)*0.6 + c*halo(d, 1.0)*0.1)*tw;
  }
  return acc;
}
