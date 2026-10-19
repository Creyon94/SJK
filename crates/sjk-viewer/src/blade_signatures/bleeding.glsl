// Bleeding Kyber: a kyber crystal fighting the dark side's corruption, as a blade (Creyon's first
// Blade Lab design, rebuilt in round 9 as Sol builds his). The blade is Sol's glow and core from
// the file, pure white, and a red corruption bleeds up it from the hilt, cracking through the
// light in jagged veins, then is pushed back down by the crystal before it rises again. Round
// it in 3D the fight goes on: red veins of corruption crawl out from the bleeding part like
// cracks in the air, drops of red light drip off it and fall, and where the white holds, beams
// of pure light flash out and push back. A swing helps the light push; a clash lets the
// corruption surge up with a red shock ring.
// @part Corruption veins
// @part Red drops
// @part Pure light
// @room 10
// @palette {"corona.rim_cool":[0.6,0.7,0.95],"corona.rim_hot":[0.95,0.97,1.0],"corona.inner":[1.0,1.0,1.0],"core.white":[1.0,1.0,1.0],"core.fringe_cool":[0.85,0.9,1.0],"core.fringe_hot":[1.0,0.95,0.95],"light.color":[1.0,0.85,0.88],"trail":[1.0,0.75,0.8]}

const vec3 BK_RED = vec3(1.0, 0.04, 0.06);
const vec3 BK_WHITE = vec3(0.92, 0.96, 1.0);
// how high the corruption has climbed (units): rising and pushed back on a slow tide
float bk_level(){
  float tide = 0.22 + 0.48*(0.5 + 0.5*sin(uTime*0.4));
  return (tide + 0.35*clamp(uClash, 0.0, 1.0) - 0.15*clamp(uSwing, 0.0, 1.0))*uLength;
}
// how corrupted the blade is at height y (with its jagged front)
float bk_mask(float y){
  float front = bk_level() + 1.5*(vn(vec2(y*0.5, uTime*0.7)) - 0.5);
  return 1.0 - smoothstep(front - 1.5, front + 0.3, y);
}

vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float lv = bk_level();
  float y0 = L*0.18, y1 = L*0.97;
  vec3 col = vec3(0.0);
  if (fxNear(ro, rd, 0.0, L) > 10.0) return col;
  float ta;
  // ---- red veins of corruption cracking out into the air from the bleeding part ----
  for (int i = 0; i < 7; i++){
    float fi = float(i);
    float k = floor(t*1.1 + h11(fi*3.1)*5.0), q = fract(t*1.1 + h11(fi*3.1)*5.0);
    float y = mix(2.0, max(lv, 3.0), h11(k*2.3 + fi));
    float a = h11(k*5.1 + fi*1.7);
    vec3 a0 = around(0.3, a, y), a1 = around(2.5 + 3.0*h11(k + fi*4.0), a + 0.08*(h11(k*7.0 + fi) - 0.5), y + (h11(k*3.3 + fi) - 0.3)*4.0);
    if (rayPt(ro, rd, (a0 + a1)*0.5, ta) > 6.0) continue;
    float grow = smoothstep(0.0, 0.25, q);
    float d = bolt3(ro, rd, a0, mix(a0, a1, grow), k + fi*1.37, 0.55);
    col += (vec3(1.0, 0.6, 0.6)*halo(d, max(px, 0.035))*0.9 + BK_RED*halo(d, 0.22)*1.0 + BK_RED*halo(d, 0.8)*0.18)*(1.0 - smoothstep(0.55, 1.0, q))*part(0.0);
  }
  // ---- drops of red light dripping off the bleeding part and falling ----
  for (int i = 0; i < 16; i++){
    float fi = float(i);
    float f = fract(t*(0.35 + 0.2*h11(fi*2.9)) + h11(fi*5.3));
    float k = floor(t*(0.35 + 0.2*h11(fi*2.9)) + h11(fi*5.3));
    float y = mix(1.0, max(lv, 2.0), h11(k*1.9 + fi));
    vec3 p = around(0.6 + 0.8*f, h11(k*4.1 + fi), y - 6.0*f*f);
    float d = rayPt(ro, rd, p, ta);
    col += BK_RED*(halo(d, max(0.08, px))*2.0 + halo(d, 0.35)*0.3)*(1.0 - smoothstep(0.7, 1.0, f))*part(1.0);
  }
  // ---- where the white holds: beams of pure light pushing back, and its motes ----
  float ya = min(lv + 2.0, y1 - 2.0);
  col += fxMotes(ro, rd, px, ya, y1, 1.0, 5.0, 30.0, 0.07, 3.6, BK_WHITE)*part(2.0);
  col += fxAura(ro, rd, ya, y1, 3.0, BK_WHITE)*0.5*part(2.0);
  // the front: a ring of struggle where red meets white
  col += (vec3(1.0)*halo(ringD(ro, rd, lv, 1.4 + 0.3*sin(t*3.0)), max(0.05, px))*1.2 + BK_RED*halo(ringD(ro, rd, lv, 1.4), 0.4)*0.6);
  // the aura: red round the bleeding part, white above
  float dlo = fxNear(ro, rd, 0.0, lv), dhi = fxNear(ro, rd, lv, L);
  col += BK_RED*halo(dlo, 2.5)*0.3 + BK_WHITE*halo(dhi, 3.0)*0.3;
  col += fxShock(ro, rd, px, lv, 1.0 - cl, 10.0, BK_RED)*1.8;
  return col*uFx;
}

// the blade: Sol's glow and core from the file, white, bleeding red from the hilt with cracks
// glowing through the corruption
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float l = max(g.r, max(g.g, g.b));
  float m = bk_mask(b.y);
  vec2 cell;
  float e = voro(vec2(b.x*3.0, b.y*0.8), cell);
  float crack = halo(e, 0.06)*m;
  vec3 c = mix(BK_WHITE, BK_RED*1.3, m*0.9);
  return l*c + BK_RED*crack*l*1.5;
}
vec3 sig_core(vec3 c, vec2 b, float px){
  float m = bk_mask(b.y);
  return c*mix(vec3(1.0), vec3(1.0, 0.55, 0.55), m*0.8);
}
