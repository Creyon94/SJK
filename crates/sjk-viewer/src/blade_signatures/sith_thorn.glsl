// Sith Thorn: a thorned stem grown in the dark side, as a blade. A deep crimson blade with sharp
// thorns of light jutting off both sides, and red petals of light drifting slowly down off it,
// turning as they fall. A clash flares it.
// @part Thorns
// @part Petals
// @room 8
// @palette {"corona.rim_cool":[0.35,0.0,0.05],"corona.rim_hot":[0.95,0.12,0.22],"corona.inner":[0.45,0.05,0.08],"core.white":[1.0,0.7,0.72],"core.fringe_cool":[0.9,0.1,0.2],"core.fringe_hot":[1.0,0.55,0.6],"light.color":[0.9,0.1,0.2],"trail":[0.9,0.1,0.2]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  return g*(0.9 + 0.15*bands(b.y, 0.05, 0.3));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 red = vec3(1.0, 0.12, 0.2);
  float s, ta;
  vec3 th = vec3(0.0), pe = vec3(0.0);
  if (fxNear(ro, rd, y0, y1) < 3.0){
    for (int i = 0; i < 9; i++){
      float fi = float(i);
      float side = mod(fi, 2.0) < 0.5 ? -1.0 : 1.0;
      float y = mix(y0, y1 - 2.0, (fi + 0.3)/9.0);
      vec3 a = vec3(side*0.35, y, 0.0);
      vec3 tip = a + vec3(side*0.9, 0.55, 0.0);
      float d = raySeg(ro, rd, a, tip, s);
      float w = mix(0.14, 0.02, s);
      th += red*halo(d, max(w, px))*0.9*(1.0 - 0.5*s) + vec3(1.0, 0.6, 0.6)*halo(d, max(w*0.4, px))*0.6;
    }
  }
  if (fxNear(ro, rd, y0 - 4.0, y1) < 3.0){
    for (int i = 0; i < 12; i++){
      float fi = float(i);
      float f = fract(t*0.06*(0.7 + 0.6*h11(fi*2.7)) + h11(fi*6.1));
      vec3 p = vec3((h11(fi*3.9) - 0.5)*3.0 + 0.8*sin(t*0.9 + fi*2.0), mix(y1, y0 - 4.0, f), 0.0);
      float d = rayPt(ro, rd, p, ta);
      vec3 q = ro + rd*ta - p;
      vec2 e = rot(t*1.2 + fi)*q.xy;
      float pd = length(e/vec2(0.22, 0.09));
      pe += red*(halo(pd - 1.0, 0.25)*0.8 + halo(d, 0.5)*0.08)*smoothstep(0.0, 0.1, f)*(1.0 - smoothstep(0.85, 1.0, f));
    }
  }
  fx += th*part(0.0) + pe*part(1.0);
  return fx*uFx;
}
