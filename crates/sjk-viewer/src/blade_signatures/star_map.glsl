// Star Map: an old Jedi star chart as a blade. Seven stars light up beside it one after another and
// lines of light link them into a route; the chart holds, fades and a new one forms. A glint sits
// on the tip. A clash flares it.
// @part Star chart
// @room 8
// @palette {"corona.rim_cool":[0.05,0.08,0.35],"corona.rim_hot":[0.35,0.45,1.0],"corona.inner":[0.8,0.85,1.0],"core.white":[0.92,0.95,1.0],"core.fringe_cool":[0.3,0.4,1.0],"core.fringe_hot":[0.8,0.86,1.0],"light.color":[0.35,0.45,1.0],"trail":[0.38,0.48,1.0]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  return g;
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 c = vec3(0.7, 0.8, 1.0);
  float ta, s;
  float cyc = t*0.12;
  float k = floor(cyc), q = fract(cyc);
  vec3 prev = vec3(0.0);
  if (fxNear(ro, rd, y0, y1) < 4.5){
    for (int i = 0; i < 7; i++){
      float fi = float(i);
      float side = h11(fi*5.7 + k*2.3) < 0.5 ? -1.0 : 1.0;
      vec3 p = vec3(side*(1.0 + 1.8*h11(fi*3.1 + k*7.7)), mix(y0 + 2.0, y1 - 2.0, (fi + h11(fi*1.3 + k))/7.0), 0.0);
      float on = smoothstep(fi/9.0, fi/9.0 + 0.05, q)*(1.0 - smoothstep(0.85, 1.0, q));
      float d = rayPt(ro, rd, p, ta);
      float tw = 0.75 + 0.25*sin(t*3.0 + fi*2.0);
      fx += (vec3(1.0)*halo(d, max(0.06, px))*1.6 + c*halo(d, 0.4)*0.4)*on*tw;
      if (i > 0){
        float dl = raySeg(ro, rd, prev, p, s);
        float grow = smoothstep(fi/9.0, fi/9.0 + 0.08, q);
        fx += c*halo(dl, max(0.025, px))*0.6*on*step(s, grow);
      }
      prev = p;
    }
  }
  return fx*uFx*part(0.0);
}
