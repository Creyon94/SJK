// Quasar: the blinding heart of a galaxy drawn as a blade. The blade is a white-hot relativistic
// jet with bright knots racing up it, each throwing a shock ring round the blade. Low on the blade
// a real tilted accretion disk of glowing matter turns and slowly wobbles round a blinding core,
// brighter on the side rushing toward you, with streams of matter spiralling inward to the heart.
// From the tip the jet blasts on up into space, widening and fading, wrapped in twisting
// magnetic light, with sparks of plasma streaking out of it. Everything pulses with the engine's heartbeat. A swing feeds the engine (the
// disk flares and the jets lengthen); a clash fires a giant knot and a shockwave off the disk.
// @room 9
// @palette {"corona.rim_cool":[0.35,0.15,0.85],"corona.rim_hot":[0.75,0.55,1.0],"corona.inner":[0.95,0.90,1.0],"core.white":[0.97,0.95,1.0],"core.fringe_cool":[0.55,0.35,1.0],"core.fringe_hot":[0.85,0.75,1.0],"light.color":[0.6,0.45,1.0],"trail":[0.65,0.5,1.0]}

float qs_top(){ return uLength*clamp(uAge/0.45, 0.0, 1.0); }
float qs_h(){ return uLength*0.32 + 0.8*sin(uTime*0.3); }
float qs_beat(){ return 0.8 + 0.2*sin(uTime*3.2) + 0.25*pow(0.5 + 0.5*sin(uTime*1.6), 8.0); }
// a point or direction moved into the disk's own frame (tilted and slowly wobbling)
vec3 qs_loc(vec3 p){
  vec2 xy = rot(0.22 + 0.06*sin(uTime*0.5))*p.xy;
  p = vec3(xy.x, xy.y, p.z);
  vec2 zy = rot(0.16*cos(uTime*0.37))*p.zy;
  return vec3(p.x, zy.y, zy.x);
}

vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, top = qs_top(), s, tq;
  float sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float beat = qs_beat()*(1.0 + 0.4*sw + 0.6*cl);
  vec3 vio = vec3(0.6, 0.38, 1.0), white = vec3(1.0, 0.97, 1.0);
  // ---- the blade: a white-hot jet in a violet sheath, from inside the hilt to the tip
  float d = raySeg(ro, rd, vec3(0.0, -0.6, 0.0), vec3(0.0, max(top, -0.5), 0.0), s);
  float y = mix(-0.6, top, s);
  float ripple = 0.8 + 0.2*sin(y*0.9 - uTime*14.0);
  vec3 light = white*exp(-pow(d/max(0.42, px), 2.0))*1.5 + vio*halo(d, 0.95)*0.7*ripple*beat + vio*halo(d, 2.8)*0.14*beat;
  vec3 fx = vec3(0.0);
  // knots racing up the jet, each with a ring of shocked light round the blade
  for (int i = 0; i < 6; i++){
    float fi = float(i);
    float ky = mod(uTime*(18.0 + 8.0*sw) + fi*(L + 8.0)/6.0, L + 8.0) - 1.0;
    if (ky > top) continue;
    float fade = smoothstep(-1.0, 1.5, ky)*(1.0 - smoothstep(L - 1.0, L, ky));
    float dk = raySeg(ro, rd, vec3(0.0, ky - 1.2, 0.0), vec3(0.0, ky + 0.4, 0.0), s);
    light += white*(exp(-pow(dk/0.55, 2.0))*1.4 + halo(dk, 1.3)*0.3)*fade;
    vec3 hr = rayAt(ro, rd, ky - 0.9);
    if (hr.z > 0.0){
      float rr = length(hr.xy);
      float ring = halo(abs(rr - 1.0), max(0.05, px*1.2))*1.3 + halo(abs(rr - 1.0), 0.35)*0.25;
      fx += mix(white, vio, 0.4)*ring*fade*(1.0 - smoothstep(1.0, 2.2, rr))*0.9;
    }
  }
  // ---- the tip jet: blasting on up from the tip, widening, with shock diamonds and knots
  float jt = smoothstep(L - 2.0, L, top);
  if (jt > 0.0){
    float JL = 7.5 + 2.0*sw + 1.0*sin(uTime*1.7);
    float dj = raySeg(ro, rd, vec3(0.0, top - 0.3, 0.0), vec3(0.0, top + JL, 0.0), s);
    float w = 0.3 + 0.9*s;
    float diamonds = 0.6 + 0.4*sin(s*JL*2.2 - uTime*20.0);
    fx += (white*exp(-pow(dj/w, 2.0))*1.2*diamonds + vio*halo(dj, w*2.5)*0.5)*pow(1.0 - s, 0.8)*1.3*jt*beat;
    // magnetic field lines twisting round the jet
    for (int k = 0; k < 2; k++){
      float fk = float(k);
      vec3 prev = around(0.35, fk*0.5 - uTime*1.2, top);
      for (int i = 1; i <= 10; i++){
        float u = float(i)/10.0;
        vec3 p = around(0.35 + 1.5*u, fk*0.5 + u*1.1 - uTime*1.2, top + u*JL*0.85);
        float dh = raySeg(ro, rd, prev, p, s);
        float uu = (float(i) - 1.0 + s)/10.0;
        fx += vio*(halo(dh, max(0.04, px))*1.4 + halo(dh, 0.25)*0.25)*(1.0 - uu)*jt*(0.6 + 0.4*sin(uu*12.0 - uTime*10.0));
        prev = p;
      }
    }
    // sparks of plasma streaking out of the tip along the jet
    for (int i = 0; i < 4; i++){
      float fi = float(i);
      float f = fract(uTime*(0.9 + 0.5*sw) + fi/4.0);
      vec3 sp = around(0.5*f*(0.5 + h11(fi*3.7 + floor(uTime*0.9 + fi/4.0))), h11(fi*1.3 + floor(uTime*0.9 + fi/4.0)), top + f*JL);
      float dk = raySeg(ro, rd, sp - vec3(0.0, 1.2 + 1.5*f, 0.0), sp, s);
      fx += white*(halo(dk, max(0.04, px))*1.4 + halo(dk, 0.3)*0.25)*(1.0 - f)*s*jt;
    }
  }
  // ---- the accretion disk round the blinding core
  float h = qs_h();
  float g = smoothstep(h - 1.0, h + 5.0, top);
  vec3 c0 = vec3(0.0, h, 0.0);
  if (g > 0.01){
    vec3 lo = qs_loc(ro - c0), ld = qs_loc(rd);
    float Ro = 6.0*g;
    vec3 m = marchSteps(lo, ld, Ro, -0.8, 0.8, 18.0);
    if (m.y > m.x){
      float spin = uTime*(1.4 + 1.2*sw);
      float t = m.x + m.z*h21(rd.xy*517.0);
      for (int i = 0; i < 18; i++){
        vec3 p = lo + ld*t;
        float r = length(p.xz)/g;
        float thick = 0.08 + 0.05*r;
        float vert = exp(-p.y*p.y/(thick*thick));
        if (vert > 0.01 && r > 0.4){
          float a = atan(p.z, p.x);
          vec2 q = rot(spin + 1.8*log(r))*p.xz;
          float n = fbm3(vec2(q.x*0.8 + 3.0, q.y*2.4));
          float arms = pow(0.5 + 0.5*sin(3.0*(a - spin) + 5.0*log(r)), 2.0);
          float rad = smoothstep(0.5, 1.0, r)*(1.0 - smoothstep(2.5, 6.0, r));
          float T = clamp(1.4/r, 0.0, 1.0);
          vec3 vt = normalize(vec3(-p.z, 0.0, p.x) + 1e-4);
          float dop = dot(vt, -ld);
          float bm = pow(1.0 + 0.55*dop, 3.0);
          float dens = rad*vert*(0.2 + 0.8*arms)*(0.3 + 1.3*smoothstep(0.3, 0.75, n));
          dens += exp(-pow((r - 0.75)/0.15, 2.0))*vert*1.6;
          fx += mix(vio, white, T*T)*dens*bm*(0.6 + 2.4*T*T)*m.z*4.0*beat;
        }
        t += m.z;
      }
    }
    // matter streaming inward on spirals across the disk, in bright pulses
    float dc = rayPt(ro, rd, c0, tq);
    if (dc < 7.0){
      for (int k = 0; k < 3; k++){
        float fk = float(k);
        float a0 = fk/3.0 - uTime*0.22*(1.0 + sw);
        vec3 prev = vec3(5.6*g*sin(6.2832*a0), 0.0, 5.6*g*cos(6.2832*a0));
        for (int i = 1; i <= 14; i++){
          float u = float(i)/14.0;
          float r = mix(5.6, 0.7, u)*g;
          float a = a0 + u*1.1;
          vec3 p = vec3(r*sin(6.2832*a), 0.06*sin(u*9.0 + uTime*3.0), r*cos(6.2832*a));
          float dh = raySeg(lo, ld, prev, p, s);
          float uu = (float(i) - 1.0 + s)/14.0;
          float pulse = 0.3 + 0.7*pow(0.5 + 0.5*sin(uu*16.0 - uTime*9.0 + fk*2.0), 4.0);
          fx += mix(vio, white, uu)*(halo(dh, max(0.035, px))*0.9 + halo(dh, 0.25)*0.4)*pulse*smoothstep(0.0, 0.2, uu)*g;
          prev = p;
        }
      }
    }
    // the blinding core: white heat round the blade at the disk's heart, beating
    light += white*(halo(dc, 0.9)*1.3 + halo(dc, 2.4)*0.35)*beat*g + vio*halo(dc, 5.5)*0.12*beat*g;
    // a clash: a shockwave ring racing out across the disk
    if (cl > 0.01){
      vec3 p0 = vec3(0.0);
      vec3 hp = vec3(0.0);
      float tt = abs(ld.y) > 1e-4 ? -lo.y/ld.y : -1.0;
      if (tt > 0.0){
        vec3 pp = lo + ld*tt;
        float rr = abs(length(pp.xz) - 1.0 - 5.5*(1.0 - cl));
        fx += white*(halo(rr, 0.12)*1.8 + halo(rr, 0.6)*0.35)*cl;
      }
      float dg = raySeg(ro, rd, vec3(0.0, L*0.55, 0.0), vec3(0.0, L*0.65, 0.0), s);
      light += white*halo(dg, 1.2)*pow(cl, 2.0)*1.6;
    }
  }
  light += fx*uFx;
  return (1.0 - exp(-light*1.1))*1.05;
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){ return vec3(0.0); }
vec3 sig_core(vec3 c, vec2 b, float px){ return vec3(0.0); }
