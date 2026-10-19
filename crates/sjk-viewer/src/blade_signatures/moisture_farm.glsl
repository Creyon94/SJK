// Moisture Farm: the dew lines of a Tatooine moisture farm as a blade. Four fine strands run up
// beside it and drops of dew slide slowly down them, glinting now and then. A clash flares it.
// @part Dew strands
// @room 8
// @palette {"corona.rim_cool":[0.2,0.26,0.48],"corona.rim_hot":[0.55,0.66,0.95],"corona.inner":[0.8,0.86,1.0],"core.white":[0.94,0.96,1.0],"core.fringe_cool":[0.5,0.6,0.92],"core.fringe_hot":[0.82,0.88,1.0],"light.color":[0.62,0.72,1.0],"trail":[0.62,0.72,1.0]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  return g;
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 c = vec3(0.75, 0.85, 1.0);
  float s, ta;
  float ym = mix(y0, y1, 0.5);
  if (fxNear(ro, rd, y0, y1) < 3.0){
    for (int k = 0; k < 4; k++){
      float fk = float(k);
      float x = (k < 2 ? -1.0 : 1.0)*((k == 0 || k == 3) ? 2.0 : 1.2);
      vec3 a = vec3(x, y0, 0.0), m = vec3(x*1.12, ym, 0.0), e = vec3(x, y1 - 1.0, 0.0);
      float d = min(raySeg(ro, rd, a, m, s), raySeg(ro, rd, m, e, s));
      fx += c*halo(d, max(0.015, px))*0.5;
      for (int i = 0; i < 3; i++){
        float fi = float(i);
        float f = fract(t*0.05*(0.8 + 0.4*h11(fk*3.0 + fi)) + h11(fk*7.0 + fi*2.3));
        float y = mix(y1 - 1.0, y0, f);
        float xx = x + x*0.12*(1.0 - abs(y - ym)/(0.5*(y1 - y0)));
        float dd = rayPt(ro, rd, vec3(xx, y, 0.0), ta);
        float tw = pow(0.5 + 0.5*sin(t*2.0 + fi*3.0 + fk), 8.0);
        fx += (vec3(1.0)*halo(dd, max(0.07, px))*(1.0 + 1.5*tw) + c*halo(dd, 0.3)*0.25)*smoothstep(0.0, 0.1, f)*(1.0 - smoothstep(0.9, 1.0, f));
      }
    }
  }
  return fx*uFx*part(0.0);
}
