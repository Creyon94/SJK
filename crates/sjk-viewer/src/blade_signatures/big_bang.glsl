// Big Bang: a blade that holds a young universe. Each ignition starts as one blinding point at the
// emitter that explodes: a sphere of light bursts outward and a fireball races up and lights the
// blade in half a second. Then the blade is a living cosmos: a white-hot core, shells of light
// rippling outward from it, soft nebula clouds swirling round it in three dimensions and a field of
// stars turning about it and drifting outward as space expands. Every so often a small new bang
// goes off somewhere along the blade, a spherical shockwave racing out. A swing stirs the gas and
// brightens the stars; a clash sets off a big bang flash.
// @room 8
// @palette {"corona.rim_cool":[0.30,0.08,0.45],"corona.rim_hot":[0.95,0.45,0.75],"corona.inner":[1.0,0.85,0.92],"core.white":[1.0,0.97,0.95],"core.fringe_cool":[0.70,0.40,1.0],"core.fringe_hot":[1.0,0.70,0.80],"light.color":[0.95,0.55,0.85],"trail":[0.90,0.50,0.85]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){ return vec3(0.0); }
vec3 sig_core(vec3 c, vec2 b, float px){ return vec3(0.0); }

// the blade lights in half a second behind a racing fireball
float bb_top(){ return uLength*smoothstep(0.04, 0.48, uAge); }
// a spherical shell of light (the ray's closest pass to its centre shows its bright rim)
float bb_shell(vec3 ro, vec3 rd, vec3 c, float R, float w, out float dc){
  float t;
  dc = rayPt(ro, rd, c, t);
  return halo(abs(dc - R), w) + 0.35*smoothstep(max(R, 0.02), 0.0, dc);
}

vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, top = bb_top(), age = uAge;
  float st = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0), fx = uFx;
  vec3 white = vec3(1.0, 0.97, 0.95), rose = vec3(1.0, 0.35, 0.72), violet = vec3(0.55, 0.3, 1.0), gold = vec3(1.0, 0.75, 0.45);
  float young = exp(-age*2.5);
  vec3 col = vec3(0.0);
  float s, ta;

  // ---- the singularity and the first burst ----
  if (age < 1.2){
    vec3 O = vec3(0.0, 0.4, 0.0);
    float dO;
    float pre = smoothstep(0.0, 0.05, age)*(1.0 - smoothstep(0.06, 0.12, age));
    float burst = smoothstep(0.05, 0.08, age)*exp(-max(age - 0.06, 0.0)*3.5);
    // the burst: an egg of light swelling out round the blade and racing up it with the fireball
    float ex = max(age - 0.05, 0.0);
    float Rh = 0.05 + 5.0*(1.0 - exp(-ex*7.0)), Ry = max(top + 2.0, Rh);
    vec3 sc = vec3(1.0, Rh/Ry, 1.0);
    vec3 rd2 = normalize(rd*sc);
    float tt;
    float dS = rayPt(ro*sc - O*sc, rd2, vec3(0.0), tt);
    float sh = halo(abs(dS - Rh), 0.3 + 0.25*Rh/5.0) + 0.35*smoothstep(Rh, 0.0, dS);
    dO = rayPt(ro, rd, O, ta);
    col += white*(halo(dO, 0.35)*pre*6.0 + exp(-dO/1.5)*pre*1.5);
    col += mix(white, rose, smoothstep(0.0, 0.8, age))*sh*burst*1.3*fx;
    // the fireball racing up the blade
    float fb = rayPt(ro, rd, vec3(0.0, top, 0.0), ta);
    col += mix(white, gold, 0.3)*(halo(fb, 0.8)*2.0 + exp(-fb/2.0)*0.6)*(1.0 - smoothstep(0.4, 0.6, age))*step(0.05, age);
  }
  if (top < 0.1) return (1.0 - exp(-col*1.1))*1.05;

  // ---- the white-hot core ----
  float d = raySeg(ro, rd, vec3(0.0, -0.6, 0.0), vec3(0.0, top, 0.0), s);
  float y = -0.6 + s*(top + 0.6);
  // mini-bangs (two at a time): heat the core where they go off
  float mb = 0.0;
  for (int i = 0; i < 2; i++){
    float fi = float(i);
    float cyc = t/2.6 + fi*0.5;
    float k = floor(cyc), q = fract(cyc)*2.6;
    float yb = (0.15 + 0.75*h11(k*3.3 + fi*7.1))*L;
    mb += exp(-pow((y - yb)/1.4, 2.0))*exp(-q*3.0);
  }
  float pul = 0.85 + 0.15*sin(t*1.7 - y*0.35);
  col += white*exp(-pow(d/max(0.36, px), 2.0))*(1.45 + mb + young + cl)*pul;
  col += mix(rose, white, 0.35)*halo(d, 0.9)*(0.6 + 0.5*mb + 0.3*st + young);
  col += rose*halo(d, 2.4)*0.25 + violet*halo(d, 2.8)*0.1*fx;

  float far = raySeg(ro, rd, vec3(0.0, -1.0, 0.0), vec3(0.0, top + 1.0, 0.0), s);
  if (fx <= 0.0 || far > 9.0) return (1.0 - exp(-col*1.1))*1.05;

  // ---- nebula gas and rippling shells (marched) ----
  vec3 ms = marchSteps(ro, rd, 5.2, -0.5, top + 2.0, 20.0);
  if (ms.y > ms.x){
    float tt = ms.x + ms.z*h21(rd.xy*517.0 + fract(t*3.0));
    vec3 acc = vec3(0.0);
    for (int i = 0; i < 20; i++){
      vec3 p = ro + rd*tt;
      tt += ms.z;
      float r = length(p.xz);
      // differential swirl: the inner gas turns faster
      vec2 q = rot(t*0.35/(0.6 + 0.3*r) + p.y*0.07)*p.xz;
      float n1 = fbm3(vec2(q.x*0.6 + p.y*0.33, q.y*0.6 - p.y*0.27) + vec2(3.1, 1.7));
      float n2 = fbm3(vec2(q.x*0.45 - q.y*0.35 + p.y*0.1, p.y*0.42 - t*0.1));
      float neb = smoothstep(0.45, 0.95, n1*0.7 + n2*0.55);
      float fall = exp(-r*r/7.0)*smoothstep(-0.5, 1.0, p.y)*(1.0 - smoothstep(top - 1.0, top + 2.0, p.y));
      // shells of light rippling outward from the core
      float sh = 0.0;
      for (int k = 0; k < 3; k++){
        float f = fract(t*0.28 + float(k)/3.0 + 0.04*sin(p.y*0.25 - t*0.6));
        float R = 0.5 + 4.6*f + 0.35*sin(p.y*0.5 - t*1.5 + float(k)*2.0)*f;
        sh += exp(-pow((r - R)/(0.18 + 0.35*f), 2.0))*(1.0 - f)*(1.0 - f);
      }
      vec3 gas = mix(mix(vec3(0.3, 0.35, 1.0), violet, smoothstep(0.3, 0.5, n2)), rose, smoothstep(0.45, 0.7, n2))*neb*1.3 + gold*neb*neb*smoothstep(2.0, 0.6, r)*0.8;
      acc += (gas*(1.0 + 0.5*st + 1.5*young) + mix(white, rose, 0.4)*sh*(0.5 + 1.4*neb))*fall;
    }
    col += acc*ms.z*0.32;
  }

  // ---- stars turning round the blade and drifting outward as space expands ----
  for (int i = 0; i < 40; i++){
    float fi = float(i);
    float e = fract(h11(fi*3.7) + t*0.025);
    float r = 0.7 + 4.4*e;
    float yy = h11(fi*1.31)*(L + 1.0);
    if (yy > top) continue;
    vec3 p = around(r, h11(fi*7.9) + t*0.12/(0.5 + 0.35*r), yy + 0.4*sin(t*0.3 + fi));
    float dd = rayPt(ro, rd, p, ta);
    float tw = 0.65 + 0.35*sin(t*(3.0 + 3.0*h11(fi)) + fi*1.7);
    float life = smoothstep(0.0, 0.12, e)*(1.0 - smoothstep(0.75, 1.0, e));
    vec3 sc = mix(white, h11(fi*2.3) < 0.5 ? vec3(0.75, 0.82, 1.0) : vec3(1.0, 0.85, 0.7), 0.5);
    col += sc*(halo(dd, max(0.06, px))*2.2 + halo(dd, 0.28)*0.18)*tw*life*(1.0 + 0.6*st);
  }

  // ---- mini-bangs: a flash and a spherical shockwave somewhere along the blade ----
  for (int i = 0; i < 2; i++){
    float fi = float(i);
    float cyc = t/2.6 + fi*0.5;
    float k = floor(cyc), q = fract(cyc)*2.6;
    float yb = (0.15 + 0.75*h11(k*3.3 + fi*7.1))*L;
    if (yb > top) continue;
    vec3 c = vec3(0.0, yb, 0.0);
    float dc;
    float R = 4.8*(1.0 - exp(-q*2.2));
    float sh = bb_shell(ro, rd, c, R, 0.12 + 0.12*R, dc);
    float fade = exp(-q*1.4)*smoothstep(0.0, 0.05, q);
    col += (mix(white, rose, smoothstep(0.0, 1.2, q))*sh*1.2 + white*halo(dc, 0.6)*exp(-q*6.0)*2.0)*fade;
  }

  // ---- clash: a big bang ----
  if (cl > 0.01){
    float a = 1.0 - cl;
    float dc;
    float sh = bb_shell(ro, rd, vec3(0.0, min(L*0.6, top), 0.0), 0.5 + 5.5*(1.0 - exp(-a*4.0)), 0.5, dc);
    col += white*(halo(dc, 1.8)*3.5 + halo(dc, 3.4)*1.2)*cl*cl + mix(white, rose, a)*sh*cl*1.4;
  }
  return (1.0 - exp(-col*1.1))*1.05;
}
