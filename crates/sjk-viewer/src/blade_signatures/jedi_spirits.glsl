// Jedi Spirits: the light of the Jedi who became one with the Force. Soft warm lights lift off the
// blade one by one and float away upward, fading; an echo of the blade sways beside it and
// afterimages linger when it swings. A clash sends a warm ring.
// @part Spirit lights
// @room 8
// @palette {"corona.rim_cool":[0.55,0.4,0.15],"corona.rim_hot":[1.0,0.85,0.55],"corona.inner":[1.0,0.97,0.88],"core.white":[1,1,0.95],"core.fringe_cool":[1.0,0.8,0.5],"core.fringe_hot":[1.0,0.93,0.75],"light.color":[1.0,0.85,0.55],"trail":[1.0,0.85,0.55]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  return g;
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 warm = vec3(1.0, 0.85, 0.55);
  for (int i = 0; i < 8; i++){
    float fi = float(i), ta;
    float f = fract(t*0.08 + h11(fi*3.7));
    vec3 p = around(0.8 + 3.5*f, h11(fi*5.3) + 0.1*sin(t*0.4 + fi), mix(y0, y1, h11(fi*2.1 + floor(t*0.08 + h11(fi*3.7)))) + 6.0*f);
    float d = rayPt(ro, rd, p, ta);
    fx += warm*(halo(d, 0.25)*1.2 + halo(d, 0.8)*0.3)*sin(3.1416*f);
  }
  fx += fxShock(ro, rd, px, L*0.55, 1.0 - cl, 8.0, warm);
  return fx*uFx*part(0.0);
}
