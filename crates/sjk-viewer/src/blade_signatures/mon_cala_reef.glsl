// Mon Cala Reef: the coral reefs of Mon Cala as a blade. Branches of glowing sea-green coral grow
// off both sides, swaying gently with pink-lit tips, and small bubbles drift up past them. A
// clash sends a ring through the water.
// @part Coral and bubbles
// @room 8
// @palette {"corona.rim_cool":[0.0,0.35,0.45],"corona.rim_hot":[0.3,0.95,0.95],"corona.inner":[0.8,1.0,1.0],"core.white":[0.92,1.0,1.0],"core.fringe_cool":[0.25,0.85,0.9],"core.fringe_hot":[0.78,1.0,1.0],"light.color":[0.3,0.9,0.95],"trail":[0.3,0.9,0.95]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  return g*(0.9 + 0.15*bands(b.y, 0.06, 0.4));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 c = vec3(0.35, 1.0, 0.95), pink = vec3(1.0, 0.5, 0.6);
  float s, ta;
  if (fxNear(ro, rd, y0, y1) < 4.0){
    for (int i = 0; i < 6; i++){
      float fi = float(i);
      float side = mod(fi, 2.0) < 0.5 ? -1.0 : 1.0;
      float y = mix(y0, y1 - 4.0, (fi + 0.5)/6.0);
      float sway = 0.15*sin(t*0.8 + fi*1.7);
      vec3 a = vec3(side*0.35, y, 0.0);
      vec3 m = a + vec3(side*1.0, 1.0, 0.0);
      vec3 e1 = m + vec3(side*(0.6 + sway), 1.1, 0.0);
      vec3 e2 = m + vec3(side*(1.1 - sway), 0.3, 0.0);
      float d = min(raySeg(ro, rd, a, m, s), min(raySeg(ro, rd, m, e1, s), raySeg(ro, rd, m, e2, s)));
      float tipd = min(rayPt(ro, rd, e1, ta), rayPt(ro, rd, e2, ta));
      fx += c*halo(d, max(0.08, px))*0.7 + c*halo(d, 0.35)*0.12 + pink*halo(tipd, 0.14)*0.9;
    }
    for (int i = 0; i < 10; i++){
      float fi = float(i);
      float f = fract(t*0.07*(0.7 + 0.6*h11(fi*2.1)) + h11(fi*5.3));
      vec3 p = vec3((h11(fi*3.7) - 0.5)*4.0 + 0.2*sin(t*1.5 + fi), mix(y0, y1, f), 0.0);
      float r = 0.12 + 0.12*h11(fi*1.9);
      float d = rayPt(ro, rd, p, ta);
      fx += vec3(0.8, 1.0, 1.0)*halo(abs(d - r), max(0.025, px))*0.7*sin(3.1416*f);
    }
  }
  return fx*uFx*part(0.0);
}
