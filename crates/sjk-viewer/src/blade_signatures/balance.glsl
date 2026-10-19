// Balance: the light side and the dark side of the Force, held in one blade (Creyon's round 9
// rework). The blade is Sol's glow and core from the file, its glow split into long waves of
// calm blue-white and crimson that roll against each other up and down it, white where they
// meet. Round it the Force itself is divided: on one side the light, a slow halo ring and
// motes of light rising serenely; on the other the dark,
// red lightning crackling out and back in, crimson sparks and dark embers whirling. The two
// sides turn slowly round the blade, and every few seconds one surges and floods across the
// other before balance returns, with a flash on the seam. A swing feeds both sides; a clash
// bursts them outward together in a white ring.
// @part Light side
// @part Dark side lightning
// @part The seam
// @room 10
// @palette {"corona.rim_cool":[0.45,0.70,1.0],"corona.rim_hot":[1.0,0.12,0.08],"corona.inner":[1.0,0.92,0.85],"core.white":[1.0,1.0,1.0],"core.fringe_cool":[0.45,0.75,1.0],"core.fringe_hot":[1.0,0.10,0.08],"light.color":[0.85,0.55,0.75],"trail":[0.9,0.45,0.6]}

const vec3 BA_BLUE = vec3(0.5, 0.75, 1.0);
const vec3 BA_GOLD = vec3(1.0, 0.92, 0.72);
const vec3 BA_RED = vec3(1.0, 0.09, 0.05);
// the light side's middle (turns round the blade); the dark side is half a turn on
float ba_spin(){ return uTime*0.05; }
// the surge: > 0 the light floods across, < 0 the dark
float ba_surge(){
  float c = uTime/7.0, k = floor(c), q = fract(c);
  float side = h11(k*3.7 + uSeed*9.0) < 0.5 ? -1.0 : 1.0;
  return side*smoothstep(0.55, 0.7, q)*(1.0 - smoothstep(0.8, 1.0, q));
}
// how much of the light side a direction at angle a (turns) is in: 1 light, 0 dark
float ba_light(float a){
  float d = abs(fract(a - ba_spin() + 0.5) - 0.5);
  return 1.0 - smoothstep(0.2 - 0.12*ba_surge(), 0.3 - 0.12*ba_surge(), d);
}
// the waves of the blade's glow at height y: 1 light, 0 dark
float ba_wave(float y){
  return smoothstep(-0.25, 0.25, sin(y*0.16 - uTime*0.9) + 0.6*ba_surge());
}

vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 col = vec3(0.0);
  if (fxNear(ro, rd, y0, y1) > 10.0) return col;
  float sp = ba_spin(), su = ba_surge();
  float lw = 1.0 + 0.8*max(su, 0.0) + 0.5*sw, dw = 1.0 + 0.8*max(-su, 0.0) + 0.5*sw;
  float s, ta;

  // its motes of light rising serenely
  for (int i = 0; i < 30; i++){
    float fi = float(i);
    float f = fract(t*(0.05 + 0.03*h11(fi*2.9)) + h11(fi*5.1));
    float a = sp + (h11(fi*1.9) - 0.5)*0.45;
    vec3 p = around(1.0 + 4.0*h11(fi*3.7), a, mix(y0 - 1.0, y1 + 2.0, f));
    float d = rayPt(ro, rd, p, ta);
    col += mix(BA_BLUE, vec3(1.0), 0.5)*(halo(d, max(0.09, px))*2.2 + halo(d, 0.45)*0.35)*sin(3.1416*f)*lw*part(0.0);
  }
  // a slow halo ring of light round the blade's upper part, brightest on the light side
  {
    float yh = L*0.72 + 1.5*sin(t*0.5);
    vec3 h = rayAt(ro, rd, yh);
    if (h.z > 0.0){
      float d = abs(length(h.xy) - 3.0);
      float a = atan(h.x, h.y)/6.2832;
      col += (vec3(1.0)*halo(d, max(0.05, px))*0.8 + BA_GOLD*halo(d, 0.3)*0.4)*ba_light(a)*lw*part(0.0);
    }
  }

  // ---- the dark side: red lightning crackling out and back, crimson sparks, embers ----
  for (int i = 0; i < 7; i++){
    float fi = float(i);
    float cyc = t*(1.6)*(0.7 + 0.6*h11(fi*1.9)) + h11(fi*3.3)*5.0, k = floor(cyc), q = fract(cyc);
    float a = sp + 0.5 + (h11(k*4.1 + fi*2.0) - 0.5)*0.42*(1.0 + 0.6*max(-su, 0.0));
    float ya = mix(y0, y1 - 3.0, h11(k*1.9 + fi*2.7)), yb = ya + 3.0 + 5.0*h11(k*2.3 + fi);
    float R = 3.0 + 3.0*h11(k*7.7 + fi);
    vec3 mid = around(R, a, (ya + yb)*0.5 + (h11(k*9.1 + fi) - 0.5)*2.0);
    if (rayPt(ro, rd, mid, ta) > R + 5.0) continue;
    float env = (1.0 - q)*(1.0 - q)*(0.6 + 0.4*step(0.5, fract(q*9.0 + fi)));
    float d = min(bolt3(ro, rd, around(0.25, a, ya), mid, k + fi*1.37, 0.5), bolt3(ro, rd, mid, around(0.25, a, yb), k + fi*1.37 + 0.5, 0.5));
    vec3 fork = mid + around(R*0.5, a + 0.1*(h11(k + fi) - 0.5), (h11(k*3.3 + fi) - 0.5)*3.0);
    float d2 = bolt3(ro, rd, mid, fork, k*2.1 + fi, 0.6);
    col += (boltLight(d, BA_RED, px) + boltLight(d2, BA_RED, px)*0.55)*env*dw*part(1.0);
  }
  for (int i = 0; i < 18; i++){
    float fi = float(i);
    float f = fract(t*(0.09 + 0.05*h11(fi*2.3)) + h11(fi*6.1));
    float a = sp + 0.5 + (h11(fi*1.3) - 0.5)*0.45 + f*0.3;
    vec3 p = around(1.2 + 4.0*h11(fi*4.7)*(0.6 + 0.4*f), a, mix(y1 + 1.0, y0 - 1.0, f));
    float d = rayPt(ro, rd, p, ta);
    float tw = 0.5 + 0.5*sin(t*9.0 + fi*2.3);
    col += mix(BA_RED, vec3(1.0, 0.45, 0.2), 0.3)*(halo(d, max(0.06, px))*2.0 + halo(d, 0.3)*0.25)*sin(3.1416*f)*tw*dw*part(1.0);
  }

  // ---- the seam: where the two sides meet, a churning line of white light up the blade ----
  for (int j = 0; j < 2; j++){
    float a = sp + 0.25 + 0.5*float(j);
    float best = 1e3, bf = 0.0;
    vec3 prev = around(1.1, a, y0);
    for (int i = 1; i <= 14; i++){
      float f = float(i)/14.0;
      vec3 p = around(1.1 + 0.4*sin(f*17.0 - t*3.0 + float(j)*2.0), a + 0.03*sin(f*11.0 + t*2.0), mix(y0, y1, f));
      float d = raySeg(ro, rd, prev, p, s);
      if (d < best){ best = d; bf = f; }
      prev = p;
    }
    float flick = 0.6 + 0.4*vn(vec2(bf*20.0 - t*6.0, float(j)*7.0));
    col += (vec3(1.0)*halo(best, max(0.04, px))*0.9 + mix(BA_BLUE, BA_RED, float(j))*halo(best, 0.35)*0.5)*flick*(0.6 + 1.4*abs(su))*part(2.0);
  }

  // the wide aura: blue on one side, crimson on the other
  float dAx = fxNear(ro, rd, y0, y1);
  vec3 hp = ro + rd*max(dot(-ro, rd), 0.0);
  float ha = atan(hp.x, hp.z)/6.2832;
  col += mix(BA_RED, BA_BLUE, ba_light(ha))*(halo(dAx, 3.0)*0.3 + halo(dAx, 1.2)*0.2);

  // a clash: both sides burst outward in one white ring
  return col*uFx;
}

// the blade: Sol's glow from the file in long waves of blue-white and crimson, white at the seams
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float l = max(g.r, max(g.g, g.b));
  float w = ba_wave(b.y);
  vec3 c = mix(BA_RED*1.25, mix(BA_BLUE, vec3(1.0), 0.2), w) + vec3(1.0, 0.9, 0.95)*4.0*w*(1.0 - w)*0.5;
  return l*c*(1.0 + 0.3*clamp(uSwing, 0.0, 1.0) + clamp(uClash, 0.0, 1.0));
}
vec3 sig_core(vec3 c, vec2 b, float px){
  float w = ba_wave(b.y);
  return c*mix(vec3(1.0, 0.8, 0.8), vec3(0.88, 0.94, 1.0), w);
}
