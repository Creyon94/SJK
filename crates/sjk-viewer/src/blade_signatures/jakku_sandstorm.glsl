// Jakku Sandstorm: a sandstorm over the Jakku wastes as a blade. Streaks of sand blow slowly across
// it from side to side, a haze of dust rolls off it and the hot air shimmers. A clash flares it.
// @part Blowing sand
// @room 8
// @palette {"corona.rim_cool":[0.55,0.35,0.12],"corona.rim_hot":[0.95,0.75,0.4],"corona.inner":[1.0,0.92,0.75],"core.white":[1.0,0.98,0.92],"core.fringe_cool":[0.95,0.72,0.35],"core.fringe_hot":[1.0,0.92,0.75],"light.color":[0.95,0.72,0.38],"trail":[0.92,0.7,0.38]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  return g*(0.9 + 0.12*bands(b.y, 0.05, 0.2));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 sand = vec3(0.95, 0.75, 0.45);
  float s;
  if (fxNear(ro, rd, y0, y1) < 6.0){
    for (int i = 0; i < 24; i++){
      float fi = float(i);
      float f = fract(t*0.18*(0.7 + 0.6*h11(fi*2.3)) + h11(fi*5.9));
      float y = mix(y0, y1, h11(fi*1.7)) + 1.5*sin(f*6.2832 + fi);
      float x = mix(-5.0, 5.0, f);
      vec3 a = vec3(x, y, 0.0);
      vec3 e = vec3(x + 0.8 + 0.6*h11(fi*3.3), y + 0.1, 0.0);
      float d = raySeg(ro, rd, a, e, s);
      fx += sand*(halo(d, max(0.04, px))*0.6 + halo(d, 0.25)*0.12)*sin(3.1416*f);
    }
  }
  return fx*uFx*part(0.0);
}
