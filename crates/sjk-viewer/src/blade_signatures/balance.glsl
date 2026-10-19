// Balance: the light side and the dark side of the Force in one blade. The blade is split down its
// length into two flows of plasma, one calm blue-white, one crimson, spiralling round each other
// all the way up like two hands clasped, so from every side and from the tip you see both. The
// border between them churns and boils; now and then one side surges and pushes the other back
// before balance returns. Red lightning crackles off the dark flow and calm pale wisps rise off
// the light flow. Swinging spins the spiral faster; a clash bursts both sides outward at once.
// @room 8
// @palette {"corona.rim_cool":[0.45,0.70,1.0],"corona.rim_hot":[1.0,0.12,0.08],"corona.inner":[1.0,0.92,0.85],"core.white":[1.0,1.0,1.0],"core.fringe_cool":[0.45,0.75,1.0],"core.fringe_hot":[1.0,0.10,0.08],"light.color":[0.85,0.55,0.75],"trail":[0.9,0.45,0.6]}

const vec3 BL_RED = vec3(1.0, 0.07, 0.04);
const vec3 BL_BLUE = vec3(0.28, 0.58, 1.0);
const vec3 BL_PALE = vec3(0.85, 0.93, 1.0);

float bl_top(){ float g = clamp(uAge/0.45, 0.0, 1.0); return uLength*(1.0 - (1.0 - g)*(1.0 - g)); }
// the spiral: the angle (turns) of the light side's middle at height y
float bl_phi(float y){ return y/13.0 - uTime*(0.32 + 0.5*uSwing); }
// how much of the blade the light side holds at height y (0.5 = balance); a surge now and then
// rolls up the blade, one side pushing the other
float bl_share(float y){
  float k = floor(uTime/6.5);
  float p = fract(uTime/6.5)*6.5;
  float dir = h11(k*3.7) < 0.5 ? -1.0 : 1.0;
  float front = (p - 0.5)*22.0;
  float surge = smoothstep(front - 8.0, front - 2.0, y)*(1.0 - smoothstep(front, front + 6.0, y));
  surge *= 1.0 - smoothstep(2.4, 3.2, p);
  return 0.5 + 0.25*dir*surge + 0.05*sin(uTime*0.9 + y*0.2);
}
// light-side weight (1 blue, 0 red) at point p, with a churning border
float bl_side(vec3 p){
  float a = atan(p.x, p.z)/6.2832;
  float u = fract(a - bl_phi(p.y) + 0.5) - 0.5;   // -0.5..0.5, 0 = middle of the light side
  float ch = (fbm3(vec2(p.y*0.7 - uTime*1.6, a*4.0 + uTime*0.3)) - 0.5)*0.22;
  float half_ = bl_share(p.y)*0.5;
  float e = half_ - abs(u + ch);
  return smoothstep(-0.03, 0.03, e);
}
// one wisp rising calmly off the light flow
vec3 bl_wisp(vec3 ro, vec3 rd, float i, float px, float top){
  float per = 2.6 + 1.2*h11(i*1.7);
  float ph = fract(uTime/per + h11(i*4.1));
  float k = floor(uTime/per + h11(i*4.1));
  float y0 = 1.5 + h11(i*9.3 + k*1.7)*(top - 4.0);
  float a0 = bl_phi(y0) + 0.08*(h11(i + k) - 0.5);
  float rise = ph*3.5;
  vec3 a = around(0.55 + ph*0.5, a0, y0 + rise*0.3);
  vec3 b = around(1.0 + ph*1.0, a0 + 0.05, y0 + 1.4 + rise);
  vec3 c = around(1.4 + ph*1.4, a0 + 0.12, y0 + 3.0 + rise*1.3);
  float s1, s2;
  float d1 = raySeg(ro, rd, a, b, s1);
  float d2 = raySeg(ro, rd, b, c, s2);
  float d = min(d1, d2);
  float sp = d1 < d2 ? s1*0.5 : 0.5 + s2*0.5;
  float life = sin(ph*3.1416);
  float shape = sin(sp*3.1416);
  return (BL_PALE*halo(d, max(px*0.8, 0.09))*0.35 + BL_BLUE*halo(d, 0.45)*0.4 + BL_BLUE*halo(d, 1.1)*0.1)*life*shape;
}
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float top = bl_top();
  float cl = clamp(uClash, 0.0, 1.0);
  vec3 col = vec3(0.0);
  float sa;
  float dax = raySeg(ro, rd, vec3(0.0, -0.6, 0.0), vec3(0.0, top, 0.0), sa);
  // the closest point to the axis decides which side's glow we see
  // each flow's own glow, from its spiralling middle line, so the two colours twine in 3D
  if (dax < 4.0){
    float dB = 1e3, dR = 1e3; float s;
    vec3 pB = around(0.42, bl_phi(-0.6), -0.6), pR = around(0.42, bl_phi(-0.6) + 0.5, -0.6);
    for (int i = 1; i <= 24; i++){
      float y = -0.6 + (top + 0.6)*float(i)/24.0;
      vec3 qB = around(0.42, bl_phi(y), y), qR = around(0.42, bl_phi(y) + 0.5, y);
      dB = min(dB, raySeg(ro, rd, pB, qB, s));
      dR = min(dR, raySeg(ro, rd, pR, qR, s));
      pB = qB; pR = qR;
    }
    float puls = 0.85 + 0.15*sin(uTime*2.0);
    col += BL_BLUE*(halo(dB, 0.9)*0.8 + halo(dB, 2.4)*0.2)*puls;
    col += BL_RED*(halo(dR, 0.9)*0.75 + halo(dR, 2.4)*0.22)*(1.9 - puls);
  }
  // a white heart where the two meet
  col += vec3(1.0)*exp(-pow(dax/max(0.08, px*0.6), 2.0))*0.6;
  // the two flows, marched through the blade's body
  if (dax < 1.0){
    vec3 ms = marchSteps(ro, rd, 0.9, -0.6, top + 0.3, 20.0);
    if (ms.z > 0.0){
      float dt = ms.z;
      float t = ms.x + dt*h21(rd.xy*517.0 + rd.z*91.0);
      vec3 acc = vec3(0.0);
      for (int i = 0; i < 20; i++){
        vec3 p = ro + rd*t;
        float r = length(p.xz);
        float w = bl_side(p);
        float body = smoothstep(0.9, 0.35, r)*smoothstep(-0.7, -0.3, p.y)*(1.0 - smoothstep(top - 0.6, top + 0.3, p.y));
        // plasma flowing up each side
        float fl = 0.75 + 0.35*vn(vec2(p.y*0.8 - uTime*5.0, atan(p.x, p.z)*2.0));
        // the churning border burns white
        float border = 4.0*w*(1.0 - w);
        vec3 c = mix(BL_RED*1.5, mix(BL_BLUE, BL_PALE, 0.2), w)*fl + vec3(1.0, 0.8, 0.9)*border*0.5;
        acc += c*body*dt*3.2;
        t += dt;
      }
      col += acc/(1.0 + length(acc)*0.2)*(1.0 + cl);
    }
  }
  // dark-side lightning crackling off the red flow
  if (dax < 5.0){
    for (int i = 0; i < 4; i++){
      float fi = float(i);
      float k = floor(uTime*9.0 + fi*0.37);
      float on = step(0.45 - 0.4*cl - 0.2*uSwing, h11(k*1.3 + fi*7.0));
      if (on < 0.5) continue;
      float y0 = 2.0 + h11(k*2.1 + fi)*(top - 5.0);
      float a0 = bl_phi(y0) + 0.5 + 0.15*(h11(k*3.3 + fi) - 0.5);
      vec3 A = around(0.5, a0, y0);
      vec3 C = around(3.0 + 2.5*h11(k*5.7 + fi) + cl*1.5, a0 + 0.12*(h11(k*7.1) - 0.5), y0 + 3.0*(h11(k*8.3 + fi) - 0.4));
      float d = bolt3(ro, rd, A, C, k + fi*11.0, 0.42);
      col += boltLight(d, BL_RED, px)*uFx*0.9;
    }
  }
  // light-side wisps
  if (dax < 4.0){
    vec3 w = vec3(0.0);
    for (int i = 0; i < 7; i++){ w += bl_wisp(ro, rd, float(i), px, top); }
    col += w*uFx;
  }
  // clash: both sides burst out, a blue ring and a red ring
  if (cl > 0.0){
    vec3 h = rayAt(ro, rd, uLength*0.6);
    if (h.z > 0.0){
      float rr = length(h.xy), aa = atan(h.x, h.y)/6.2832;
      float side = smoothstep(-0.1, 0.1, sin((aa - bl_phi(uLength*0.6))*6.2832));
      float R = 0.6 + (1.0 - cl)*5.0;
      vec3 rc = mix(BL_RED, BL_BLUE, side);
      col += (rc*halo(abs(rr - R), 0.45) + vec3(1.0)*halo(abs(rr - R), 0.1))*cl*1.6*uFx;
    }
  }
  return (1.0 - exp(-col*1.1))*1.05;
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){ return vec3(0.0); }
vec3 sig_core(vec3 c, vec2 b, float px){ return vec3(0.0); }
