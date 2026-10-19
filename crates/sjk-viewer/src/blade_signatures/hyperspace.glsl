// Hyperspace: the jump to lightspeed, as a blade. The blade is a blue-white shaft of light that
// shudders with the strain of the jump. Round it, at several distances, the stars are stretched
// into streaks rushing up past the blade toward the tip, the nearer ones faster, inside a blue
// tunnel of light that twists round the blade like the hyperspace corridor. Seen from above the
// streaks fly out of the tip like a starfield. Ignition is the jump itself: the stars sit as points,
// then stretch into lines. A swing punches the jump harder: the streaks grow long and bright and a
// flash races up. A clash drops out of hyperspace: a white reversion flash and a ring of light.
// @room 8
// @palette {"corona.rim_cool":[0.08,0.25,0.85],"corona.rim_hot":[0.45,0.75,1.0],"corona.inner":[0.85,0.95,1.0],"core.white":[0.96,0.98,1.0],"core.fringe_cool":[0.25,0.55,1.0],"core.fringe_hot":[0.7,0.88,1.0],"light.color":[0.35,0.6,1.0],"trail":[0.4,0.65,1.0]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){ return vec3(0.0); }
vec3 sig_core(vec3 c, vec2 b, float px){ return vec3(0.0); }

// how stretched the stars are: 0 points, 1 full streaks (the jump on ignition, a drop on a clash)
float hy_stretch(){
  float jump = smoothstep(0.05, 0.55, uAge);
  float drop = 1.0 - smoothstep(0.0, 0.5, uClash)*0.85;
  return jump*drop*(1.0 + 1.2*clamp(uSwing, 0.0, 1.0));
}

vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime;
  float sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  vec3 blue = vec3(0.25, 0.5, 1.0), ice = vec3(0.7, 0.86, 1.0), white = vec3(1.0);
  float top = L*clamp(uAge/0.45, 0.0, 1.0);
  // ---- the blade: a blue-white shaft that shudders (the tip trembles, the light flickers)
  float sh = 0.05 + 0.06*sw;
  vec3 tipOff = vec3(sin(t*61.0) + 0.5*sin(t*97.0 + 1.3), 0.0, cos(t*53.0) + 0.5*sin(t*83.0))*sh;
  float sb;
  float db = raySeg(ro, rd, vec3(0.0, -0.6, 0.0), vec3(0.0, top, 0.0) + tipOff, sb);
  float fl = 0.9 + 0.1*sin(t*37.0) + 0.06*sin(t*71.0 + sb*9.0);
  float flash = sw*0.5 + cl*1.2;
  vec3 light = white*exp(-pow(db/max(0.33, px), 2.0))*1.5*fl
             + mix(blue, ice, 0.35)*halo(db, 0.8)*0.75*fl
             + blue*halo(db, 2.4)*(0.2 + 0.25*flash);
  float ta;
  float axd = rayPt(ro, rd, vec3(0.0, clamp(ro.y + rd.y*dot(-ro, rd), 0.0, L), 0.0), ta);
  // nearest approach of the ray to the axis segment (for culling and depth)
  float sa;
  float near = raySeg(ro, rd, vec3(0.0, 0.0, 0.0), vec3(0.0, L, 0.0), sa);
  float fx = uFx;
  if (near < 7.0 && fx > 0.0){
    float S = hy_stretch();
    vec3 fxl = vec3(0.0);
    // ---- the streaks: stars on cylinders round the blade, rushing up
    for (int i = 0; i < 36; i++){
      float fi = float(i);
      float layer = floor(fi/12.0);                       // 0 inner, 1 middle, 2 outer
      float r = 1.3 + 1.4*layer + 0.9*h11(fi*1.37);
      float nearF = 1.0 - layer*0.3;
      float speed = (26.0 + 12.0*h11(fi*5.1))*nearF;
      float span = L + 14.0;
      float ph = t*speed/span + h11(fi*9.13);
      float cyc = floor(ph);
      float y = fract(ph)*span - 6.0;                    // the head
      float a = h11(fi*3.71 + cyc*1.618) + fi/12.0;
      float rr = r + 0.6*(h11(cyc*2.3 + fi) - 0.5);
      float len = 0.15 + (2.5 + 5.0*nearF)*S*(1.0 + 0.6*sw);
      vec3 hd = around(rr, a, y);
      vec3 tl = around(rr, a, y - len);
      float s;
      float d = raySeg(ro, rd, tl, hd, s);
      if (d > 0.8) continue;
      float ym = y - len*(1.0 - s);
      float on = smoothstep(-1.0, 3.0, ym)*(1.0 - smoothstep(top - 4.0, top + 2.0, ym));
      float w = max(0.035 + 0.025*nearF, px*0.8);
      vec3 c = mix(blue, white, 0.35 + 0.65*s);
      fxl += c*(halo(d, w)*1.7 + halo(d, 0.22)*0.25)*(0.3 + 0.7*s)*on*(0.6 + 0.4*nearF)*(1.0 + 0.8*sw);
    }
    // ---- the tunnel: a twisting blue corridor of light round the blade
    vec3 m = marchSteps(ro, rd, 5.4, -0.5, top + 1.5, 16.0);
    if (m.y > m.x){
      float tt = m.x + m.z*h21(rd.xy*517.0 + ro.xz*3.1);
      float acc = 0.0;
      for (int k = 0; k < 16; k++){
        vec3 p = ro + rd*tt;
        float rad = length(p.xz);
        float shell = exp(-pow((rad - 3.3)/1.0, 2.0));
        vec2 q = rot(p.y*0.13 - t*1.6) * p.xz;
        float n = fbm3(q*0.55 + vec2(0.0, p.y*0.05 - t*0.6));
        float streak = fbm3(q*1.1 + vec2(p.y*0.09 - t*3.0, n*2.0));
        float band = pow(0.5 + 0.5*sin(3.0*atan(p.z, p.x) + p.y*0.28 - t*4.0 + n*2.5), 2.5);
        float dn = shell*(pow(clamp(n*0.5 + streak*0.6 - 0.4, 0.0, 1.0), 1.6)*0.6 + band*streak*1.1);
        dn *= smoothstep(-0.5, 3.0, p.y)*(1.0 - smoothstep(top - 5.0, top + 1.5, p.y));
        acc += dn*m.z;
        tt += m.z;
      }
      fxl += vec3(0.16, 0.38, 1.0)*acc*(0.17 + 0.06*sw)*min(S, 1.0);
    }
    // ---- a swing's flash racing up the blade
    float yf = fract(t*1.3)*(L + 8.0) - 4.0;
    float sy = ro.y + rd.y*ta;
    fxl += ice*halo(abs(sy - yf), 2.0)*halo(axd, 1.6)*sw*1.2;
    // ---- a clash: the reversion flash and a ring of light expanding round the blade
    if (cl > 0.01){
      vec3 o = vec3(0.0, L*0.55, 0.0);
      float to;
      float dc = rayPt(ro, rd, o, to);
      float R = 0.8 + 5.0*(1.0 - cl);
      float shellL = halo(R - dc, 0.25 + 0.4*(1.0 - cl))*smoothstep(R + 0.3, R - 0.2, dc);
      fxl += white*shellL*cl*1.1 + ice*halo(dc, 3.0)*pow(cl, 3.0)*1.4;
      vec3 h = rayAt(ro, rd, o.y);
      if (h.z > 0.0) fxl += white*halo(abs(length(h.xy) - R*1.1), 0.15 + 0.3*(1.0 - cl))*cl*1.4;
    }
    light += fxl*fx;
  }
  return (1.0 - exp(-light*1.1))*1.05;
}
