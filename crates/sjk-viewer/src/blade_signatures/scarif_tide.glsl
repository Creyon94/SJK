// Scarif Tide: the turquoise seas of Scarif as a blade. Bands of shallow-sea light roll up it;
// turquoise waves roll up beside it, crest after crest, breaking into foam as they reach its
// upper part (round 15: the waves replaced currents it shared with the Aurodium), a fine sea spray
// glitters off it. A clash sends a wave out.
// @part Waves
// @part Sea spray
// @room 8
// @palette {"corona.rim_cool":[0.0,0.4,0.45],"corona.rim_hot":[0.3,0.95,0.9],"corona.inner":[0.85,1.0,0.98],"core.white":[1,1,1],"core.fringe_cool":[0.2,0.85,0.85],"core.fringe_hot":[0.75,1.0,0.97],"light.color":[0.3,0.9,0.85],"trail":[0.3,0.9,0.85]}

vec3 foam_c(){ return vec3(0.85, 1.0, 1.0); }
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float caust = pow(0.5 + 0.5*sin(b.y*0.7 - uTime*1.5 + 2.0*sin(b.x*5.0 + uTime)), 3.0);
  return g*(0.8 + 0.3*bands(b.y, 0.08, 1.0) + 0.4*caust);
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 sea = vec3(0.25, 0.95, 0.9), foam = vec3(0.85, 1.0, 1.0);
  float s;
  if (fxNear(ro, rd, y0, y1) < 4.5){
    for (int k = 0; k < 4; k++){
      float fk = float(k);
      float q = fract(t*0.16 + fk/4.0);
      float yc = mix(y0, y1, q);
      float best = 1e3;
      vec3 prev = vec3(-3.0, yc, 0.0);
      for (int i = 1; i <= 12; i++){
        float x = -3.0 + 6.0*float(i)/12.0;
        // a crest, steeper as it climbs, leaning forward
        float h = 0.45*sin(x*1.6 - t*1.8 + fk*2.0) + 0.5*q*exp(-x*x*0.6);
        vec3 p = vec3(x, yc + h, 0.0);
        best = min(best, raySeg(ro, rd, prev, p, s));
        prev = p;
      }
      float env = sin(3.1416*q);
      float foam = smoothstep(0.55, 0.9, q);
      fx += (mix(sea, foam_c(), foam)*halo(best, 0.12 + 0.1*foam)*0.9 + vec3(1.0)*halo(best, max(0.025, px))*(0.3 + 0.7*foam)
             + sea*halo(best, 0.6)*0.12)*env*part(0.0);
    }
  }
  fx += fxMotes(ro, rd, px, y0, y1, 0.8, 3.5, 22.0, 0.06, 6.1, foam)*0.7*part(1.0);
  return fx*uFx;
}
