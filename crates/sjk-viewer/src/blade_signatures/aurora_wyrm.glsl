// Ilum Aurora Storm: the polar sky of Ilum, the kyber world, as a blade. A pale-green blade under
// curtains of aurora that hang round it in 3D: tall ribbons of green fading up to magenta, waving
// slowly and rippling as light runs along them; snow crystals glitter as they whirl round it and
// kyber glints flash in the cold. Every twelve seconds a storm surge sweeps up the blade and the
// curtains flare and spread wide. A swing makes the curtains ripple; a clash flashes them all.
// @part Aurora curtains
// @arcs Storm lightning
// @motes Ice crystals
// @hue Aurora colours
// @room 10
// @palette {"corona.rim_cool":[0.05,0.30,0.25],"corona.rim_hot":[0.25,0.95,0.60],"corona.inner":[0.75,1.0,0.88],"core.white":[0.92,1.0,0.95],"core.fringe_cool":[0.25,0.85,0.60],"core.fringe_hot":[0.75,1.0,0.85],"light.color":[0.30,0.95,0.60],"trail":[0.35,0.95,0.65]}

float ia_top(){ return uLength*clamp(uAge/0.45, 0.0, 1.0); }
float ia_surge(){ float p = mod(uTime - 4.0, 12.0); return smoothstep(0.0, 1.0, p)*(1.0 - smoothstep(3.0, 5.0, p)); }
vec3 r8_vol(vec3 ro, vec3 rd, float px){
  float t = uTime, top = ia_top(), s, ta;
  float sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  vec3 green = vec3(0.25, 1.0, 0.55), mag = vec3(0.95, 0.3, 0.75);
  vec3 col = vec3(0.0);
  if (top < 0.5) return col;
  float su = ia_surge();
  // aurora curtains: each a band of tall rays standing on an arc round the blade
  for (int k = 0; k < 3; k++){
    float fk = float(k);
    float R = (2.2 + 0.9*fk)*(1.0 + 0.4*su);
    float y0 = uLength*(0.15 + 0.22*fk);
    float H = 7.0 + 3.0*su;
    float a0 = fk*0.33 + t*0.02*(mod(fk, 2.0) < 0.5 ? 1.0 : -1.0);
    for (int j = 0; j < 9; j++){
      float u = float(j)/8.0;
      float a = a0 + u*0.35;
      float wav = 0.25*sin(u*7.0 + t*(0.8) + fk*2.0);
      vec3 base = around(R + wav, a, y0 + 0.6*sin(u*5.0 + t*0.5));
      if (base.y > top) continue;
      vec3 tip = base + vec3(0.0, H*(0.6 + 0.4*sin(u*9.0 + fk + t*0.7)), 0.0);
      float d = raySeg(ro, rd, base, tip, s);
      float run = 0.5 + 0.5*sin(s*8.0 - t*3.0 + u*6.0);
      float edge = sin(3.1416*u);
      vec3 c = mix(green, mag, smoothstep(0.35, 1.0, s));
      col += c*(halo(d, 0.22)*1.4 + halo(d, 0.7)*0.35)*(0.4 + 0.6*run)*(1.0 - s)*(0.3 + 0.7*edge)*(1.0 + 1.2*su + 1.5*cl);
    }
  }
  // snow crystals whirling round it
  for (int i = 0; i < 26; i++){
    float fi = float(i);
    float r = 0.9 + 3.6*h11(fi*3.3);
    vec3 p = around(r, h11(fi*7.7) + t*(0.2 + 0.2*h11(fi)), mod(h11(fi*5.9)*(top + 2.0) - t*(0.8 + 0.8*h11(fi*2.4)), top + 2.0) - 1.0);
    float d = rayPt(ro, rd, p, ta);
    float gl = pow(0.5 + 0.5*sin(t*6.0 + fi*3.1), 8.0);
    col += vec3(0.9, 1.0, 1.0)*halo(d, max(0.04, px))*(0.8 + 1.6*gl);
  }
  // kyber glints flashing in the cold
  for (int i = 0; i < 4; i++){
    float fi = float(i);
    float k = floor(t*0.8 + fi*0.25);
    vec3 p = around(0.6 + 1.5*h11(k*2.1 + fi), h11(k*3.7 + fi), (0.1 + 0.8*h11(k*5.3 + fi))*top);
    float f = fract(t*0.8 + fi*0.25);
    float d = rayPt(ro, rd, p, ta);
    col += vec3(0.85, 1.0, 0.95)*(halo(d, 0.08 + px)*3.0 + halo(d, 0.5)*0.4)*exp(-f*5.0);
  }
  return col*uFx;
}
// the blade: Sol's glow and core from the file, a storm surge of light sweeping up it
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float p = mod(uTime - 4.0, 12.0);
  float sweep = halo(b.y - p*14.0 + 2.0, 3.0)*step(p, 3.5);
  return g*(0.9 + 0.1*sin(b.y*0.5 - uTime*1.5) + 1.2*sweep + 0.5*clamp(uClash, 0.0, 1.0));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }

// ---- round 9: the big effects layer ----
// Round 11 (Creyon): its own look, Sol's way: its round-8 effects as part 0, its own parts
// (`part(k)`, the `// @part` lines) and Sol's pieces from its file; nothing shared with the others.
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 col = r8_vol(ro, rd, px)*part(0.0);
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  fx += fxAura(ro, rd, y0, y1, 3.0, vec3(0.3, 1.0, 0.6))*0.8*part(0.0);
  return col + fx*uFx;
}
