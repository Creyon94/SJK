// Felucia Spores: the glowing fungal jungle of Felucia as a blade. A warm amber-green blade
// breathing with bioluminescent rings that pulse up it; clouds of glowing spores swirl round it
// resting on it while it is still and flying off in spirals when it swings, settling back after; luminous filaments wave off its surface and now and
// then a spore pod bursts off it in a puff of light that rains spores. A swing whirls the spores
// into a vortex; a clash bursts a huge cloud of them.
// @part Spores and filaments
// @room 10
// @palette {"corona.rim_cool":[0.30,0.35,0.02],"corona.rim_hot":[0.95,0.80,0.15],"corona.inner":[1.0,0.95,0.60],"core.white":[1.0,0.98,0.85],"core.fringe_cool":[0.70,0.85,0.20],"core.fringe_hot":[1.0,0.92,0.50],"light.color":[0.90,0.85,0.30],"trail":[0.90,0.85,0.35]}

float fs_top(){ return uLength*clamp(uAge/0.45, 0.0, 1.0); }
vec3 fs_col(float k){ return k < 0.33 ? vec3(1.0, 0.75, 0.15) : k < 0.66 ? vec3(0.7, 1.0, 0.25) : vec3(0.25, 0.95, 0.8); }
vec3 base_vol(vec3 ro, vec3 rd, float px){
  float t = uTime, top = fs_top(), s, ta;
  float sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  vec3 col = vec3(0.0);
  if (top < 0.5) return col;
  // spores: glowing spores resting on the blade, breathing softly while it is still; a swing
  // shakes them off and they fly out round it in spirals, trailing light, then settle back on
  // it as it calms (Creyon's round 9, like the butterflies)
  float fly = sqrt(sw);
  for (int i = 0; i < 44; i++){
    float fi = float(i);
    float yr = (0.06 + 0.9*h11(fi*3.1))*top;
    float ar = h11(fi*1.3);
    float rest = 0.75 + 0.25*h11(fi*2.7);
    float bob = 0.05*sin(t*1.7 + fi*2.1);
    float out = fly*(2.0 + 4.0*h11(fi*7.7));
    float spin = fly*(0.35 + 0.5*h11(fi*5.1))*(h11(fi*9.3) < 0.5 ? -1.0 : 1.0);
    float a = ar + spin + 0.05*sin(t*0.9 + fi)*fly;
    vec3 p = around(rest + bob + out, a, yr + fly*2.5*(h11(fi*4.1) - 0.35) + 0.6*sin(t*2.0 + fi)*fly);
    vec3 p0 = around(rest + bob + out*0.92, a - spin*0.06, p.y - 0.25*fly);
    float d = fly > 0.05 ? raySeg(ro, rd, p0, p, s) : rayPt(ro, rd, p, ta);
    float tw = 0.55 + 0.45*sin(t*(2.0 + 2.0*h11(fi)) + fi*2.0);
    vec3 sc = fs_col(h11(fi*8.8));
    col += (vec3(1.0)*halo(d, max(0.06, px))*(0.8 + 0.8*tw) + sc*halo(d, max(0.16, px))*(1.4 + 1.2*tw) + sc*halo(d, 0.5)*0.3)*(0.9 + 0.5*fly);
  }
  // drifting spore dust round it all the time
  col += fxMotes(ro, rd, px, top*0.1, top, 1.0, 5.0, 24.0, 0.05, 2.7, vec3(0.8, 1.0, 0.4));
  // luminous filaments waving off the blade
  for (int i = 0; i < 10; i++){
    float fi = float(i);
    float y0 = (0.08 + 0.85*h11(fi*2.9))*top;
    float a = h11(fi*6.1);
    vec3 prev = around(0.4, a, y0);
    float best = 1e3, bu = 0.0;
    for (int j = 1; j <= 5; j++){
      float u = float(j)/5.0;
      vec3 q = around(0.4 + 1.6*u, a + 0.06*sin(t*1.5 + fi + u*3.0), y0 + 0.8*u + 0.4*sin(t*2.0 + fi*1.7 + u*4.0)*u);
      float d = raySeg(ro, rd, prev, q, s);
      if (d < best){ best = d; bu = (float(j) - 1.0 + s)/5.0; }
      prev = q;
    }
    col += fs_col(h11(fi*4.4))*(halo(best, max(0.03, px))*0.9*(1.0 - bu) + halo(best, 0.18)*0.15) + vec3(1.0, 1.0, 0.8)*halo(length(vec2(best, 0.0)), 0.06)*step(0.9, bu)*1.2;
  }
  // spore pods bursting off it in a puff of light that rains spores
  for (int i = 0; i < 2; i++){
    float fi = float(i);
    float f = fract(t*0.3 + fi*0.5);
    float k = floor(t*0.3 + fi*0.5);
    vec3 c = around(0.6 + 1.0*f, h11(k*3.7 + fi), (0.2 + 0.6*h11(k*1.9 + fi))*top);
    float d = rayPt(ro, rd, c, ta);
    col += vec3(1.0, 0.95, 0.6)*(halo(d, 0.4 + 2.0*f)*0.5*(1.0 - f) + halo(abs(d - 3.0*f), 0.15)*0.6*(1.0 - f));
  }
  // a clash: a huge cloud of spores
  if (cl > 0.01){
    for (int i = 0; i < 16; i++){
      float fi = float(i);
      vec3 dir = normalize(vec3(sin(fi*2.4), 0.8*(h11(fi*3.0) - 0.4), cos(fi*2.4)));
      vec3 p = vec3(0.0, uLength*0.6, 0.0) + dir*(0.6 + 5.0*(1.0 - cl));
      float d = rayPt(ro, rd, p, ta);
      col += fs_col(h11(fi))*halo(d, max(0.08, px))*2.0*cl;
    }
  }
  // round 9: the jungle's light: soft beams of bioluminescence and a living aura
  float y0 = top*0.2, y1 = top*0.97;
  col += fxAura(ro, rd, y0, y1, 3.0, vec3(0.8, 0.9, 0.3));
  col += fxShock(ro, rd, px, uLength*0.6, 1.0 - cl, 9.0, vec3(0.9, 1.0, 0.5))*1.5;
  return col*uFx;
}
// the blade: Sol's glow and core from the file, bioluminescent rings pulsing up it
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float ring = pow(0.5 + 0.5*sin(b.y*0.9 - uTime*3.0), 6.0);
  return g*(0.85 + 0.6*ring + 0.5*clamp(uClash, 0.0, 1.0));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// round 11: sig_vol gated by part 0
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  if (part(0.0) < 0.5) { return vec3(0.0); }
  return base_vol(ro, rd, px);
}
