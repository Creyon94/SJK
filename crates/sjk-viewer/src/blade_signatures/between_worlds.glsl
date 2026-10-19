// Between Worlds: a blade from the World Between Worlds, the path of light through the void
// outside time. A calm white-gold blade stands in its own small starry void. Faint echoes of the
// blade from other moments drift beside it in depth, translucent, fading in and out like ripples
// of time. Voices of light, soft streams, flow along it from both ends, from the hilt up and from
// the tip down, passing each other. Soft portal rings of shimmering light open and close along it,
// and star dust drifts slowly through the dark. A swing scatters the echoes wide; a clash makes
// every portal flash open at once.
// @room 8
// @palette {"corona.rim_cool":[0.42,0.34,0.16],"corona.rim_hot":[1.0,0.86,0.55],"corona.inner":[1.0,0.96,0.85],"core.white":[1.0,1.0,0.97],"core.fringe_cool":[0.90,0.78,0.50],"core.fringe_hot":[1.0,0.90,0.65],"light.color":[1.0,0.90,0.66],"trail":[1.0,0.90,0.68]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){ return vec3(0.0); }
vec3 sig_core(vec3 c, vec2 b, float px){ return vec3(0.0); }

// a soft ring (torus) of radius R at height h, thickness w: its density integrated along the ray
float bww_ring(vec3 ro, vec3 rd, float h, float R, float w){
  vec2 sp = raySpan(ro, rd, R + 2.5*w, h - 2.5*w, h + 2.5*w);
  if (sp.y <= sp.x) return 0.0;
  float dt = (sp.y - sp.x)/6.0;
  float t = sp.x + dt*0.5;
  float acc = 0.0;
  for (int i = 0; i < 6; i++){
    vec3 p = ro + rd*t;
    float r = length(p.xz);
    float a = atan(p.x, p.z);
    float d2 = (r - R)*(r - R) + (p.y - h)*(p.y - h)*0.5;
    acc += exp(-d2/(w*w))*dt*(0.7 + 0.3*sin(a*5.0 + uTime*1.3 + h));
    t += dt;
  }
  return acc;
}

vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength;
  float sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float top = L*clamp(uAge/0.45, 0.0, 1.0);
  float fxOn = uFx*smoothstep(0.3, 1.2, uAge);
  vec3 gold = vec3(1.0, 0.80, 0.45);
  vec3 pale = vec3(1.0, 0.95, 0.85);
  vec3 dusk = vec3(0.55, 0.45, 0.30);

  // the blade: a calm white-gold path of light
  float s;
  float dA = raySeg(ro, rd, vec3(0.0, -0.6, 0.0), vec3(0.0, top, 0.0), s);
  float ya = mix(-0.6, top, s);
  float br = 0.85 + 0.15*sin(ya*0.5 - uTime*0.8) + 0.10*sin(ya*0.21 + uTime*0.5);
  vec3 light = vec3(1.0)*exp(-pow(dA/max(0.24, px), 2.0))*(1.35 + cl*0.5)
    + gold*halo(dA, 0.55)*0.75*br
    + gold*halo(dA, 1.8)*0.22*br
    + dusk*halo(dA, 5.0)*0.06;

  if (fxOn < 0.001) return light;

  // echoes of the blade from other moments: faint copies offset in depth, drifting and fading
  for (int e = 0; e < 4; e++){
    float fe = float(e);
    float per = 6.0 + 1.7*fe;
    float f = fract(uTime/per + fe*0.27);
    float vis = smoothstep(0.0, 0.25, f)*(1.0 - smoothstep(0.55, 1.0, f));
    float ang = h11(fe*3.3 + floor(uTime/per + fe*0.27)*1.7);
    float off = (0.9 + 1.6*f + 2.5*sw)*(1.0 - 0.6*cl);
    vec3 a0 = around(off*0.35, ang, 0.6);
    vec3 a1 = around(off, ang + 0.06*sin(uTime*0.3 + fe), min(top, L*(0.92 + 0.05*sin(uTime*0.4 + fe))));
    float se;
    float de = raySeg(ro, rd, a0, a1, se);
    float tf = smoothstep(0.0, 0.08, se)*(1.0 - smoothstep(0.85, 1.0, se));
    float flick = 0.8 + 0.2*sin(se*30.0 - uTime*3.0 + fe);
    vec3 ec = e == 1 ? mix(gold, vec3(1.0, 0.65, 0.30), 0.6) : (e == 3 ? mix(pale, vec3(0.75, 0.85, 1.0), 0.5) : pale);
    light += ec*(exp(-pow(de/max(0.16, px), 2.0))*0.55 + halo(de, 0.6)*0.22)*vis*tf*flick;
  }

  // voices of light: soft streams winding loosely along the blade, two rising from the hilt and
  // two falling from the tip, glowing pulses travelling along them
  for (int k = 0; k < 4; k++){
    float fk = float(k);
    float dir = k < 2 ? 1.0 : -1.0;
    float rr = 0.75 + 0.35*fk;
    float turns = (0.55 + 0.18*fk)*dir;
    float ph = fk*0.31 + uTime*0.04*dir;
    vec3 prev = around(rr*0.4, ph, 0.0);
    for (int j = 1; j <= 16; j++){
      float fj = float(j)/16.0;
      float y = fj*L;
      float rj = rr*(0.4 + 0.6*sin(min(fj*1.6, 1.0)*1.5708))*(1.0 - 0.5*smoothstep(0.85, 1.0, fj)) + 0.10*sin(fj*4.0 + uTime*0.5 + fk);
      vec3 q = around(rj, ph + fj*turns, y);
      if (y - L/16.0 < top){
        float sg;
        float d = raySeg(ro, rd, prev, q, sg);
        if (d < 2.0){
          float u = (fj - (1.0 - sg)/16.0);
          // travel: rising streams move up, falling ones down
          float pos = dir > 0.0 ? u : 1.0 - u;
          float pulse = pow(0.5 + 0.5*sin(pos*14.0 - uTime*(2.2 + 2.0*sw) + fk*1.7), 3.0);
          float vis = smoothstep(0.0, 0.12, pos)*(1.0 - smoothstep(0.8, 1.0, pos));
          vec3 vc = dir > 0.0 ? gold : pale;
          float w = max(px*0.8, 0.05);
          light += (vec3(1.0)*halo(d, w)*0.45*pulse + vc*halo(d, max(w*2.5, 0.14))*(0.10 + 0.45*pulse) + vc*halo(d, 0.7)*0.06*(0.3 + pulse))*vis;
        }
      }
      prev = q;
    }
  }

  // portals: soft shimmering rings opening and closing along the blade, a veil of light inside
  for (int k = 0; k < 3; k++){
    float fk = float(k);
    float per = 5.5 + 1.9*fk;
    float f = fract(uTime/per + fk*0.37);
    float open = sin(clamp(f/0.7, 0.0, 1.0)*3.1416);
    open = max(open, cl*0.45);
    float h = L*(0.25 + 0.27*fk) + 1.2*sin(uTime*0.25 + fk*2.0);
    if (h < top && open > 0.01){
      float R = 0.6 + 2.4*open + 4.0*cl*(1.0 - cl);
      float ring = bww_ring(ro, rd, h, R, 0.35 + 0.35*open);
      light += mix(gold, pale, 0.5)*ring*open*(0.6 + 0.5*cl)*(0.35 + 0.65*smoothstep(0.05, 0.5, abs(rd.y)));
      vec3 hp = rayAt(ro, rd, h);
      if (hp.z > 0.0){
        float r = length(hp.xy);
        float a = atan(hp.x, hp.y);
        float veil = exp(-pow(r/(R*0.85), 4.0))*(0.5 + 0.5*sin(r*5.0 - uTime*3.0 + 0.6*sin(a*3.0 + uTime)));
        float gz = clamp(abs(rd.y), 0.15, 1.0);
        light += pale*veil*open*0.07/gz*smoothstep(0.3, 0.9, r);
      }
    }
  }

  // the starry void: tiny stars round the blade, some slowly drifting, twinkling
  for (int i = 0; i < 34; i++){
    float fi = float(i);
    float rr = 1.6 + 3.8*h11(fi*2.7);
    float a = h11(fi*6.1) + uTime*0.004*(h11(fi*4.4) - 0.5);
    float y = mod(L*h11(fi*8.3) + uTime*(0.15 + 0.25*h11(fi*1.1))*(i < 17 ? 1.0 : -1.0), L + 2.0) - 0.5;
    vec3 q = around(rr, a, y);
    if (q.y < top){
      float tt; float d = rayPt(ro, rd, q, tt);
      float tw = 0.4 + 0.6*pow(0.5 + 0.5*sin(uTime*(1.3 + 2.0*h11(fi*5.5)) + fi*3.0), 2.0);
      float edge = smoothstep(-0.5, 1.0, y)*(1.0 - smoothstep(L - 1.0, L + 1.5, y));
      float w = max(px*0.9, 0.035);
      vec3 sc = h11(fi*9.9) > 0.6 ? vec3(0.80, 0.86, 1.0) : pale;
      light += (sc*halo(d, w)*1.4 + sc*halo(d, w*3.5)*0.18)*tw*edge;
    }
  }

  // star dust and ripples of time: a faint marched haze round the blade, rings of rippling light
  vec3 ms = marchSteps(ro, rd, 5.6, -0.3, L + 1.5, 16.0);
  if (ms.y > ms.x){
    float t = ms.x + ms.z*h21(rd.xy*517.0 + rd.z*29.0);
    float acc = 0.0;
    for (int i = 0; i < 16; i++){
      vec3 p = ro + rd*t;
      if (p.y < top){
        float r = length(p.xz);
        float n = fbm3(vec2(p.x*0.35 + p.y*0.05, p.y*0.10 + uTime*0.04))*fbm3(vec2(p.z*0.35 - p.y*0.04 + 3.0, p.y*0.12 - uTime*0.05));
        float rip = 0.55 + 0.45*sin(r*2.2 - uTime*1.1 + p.y*0.15);
        float sh = smoothstep(0.6, 2.2, r)*(1.0 - smoothstep(2.8, 5.4, r))*smoothstep(-0.3, 2.0, p.y)*(1.0 - smoothstep(L - 1.5, L + 1.5, p.y));
        acc += smoothstep(0.28, 0.6, n*1.4)*sh*rip;
      }
      t += ms.z;
    }
    light += mix(dusk, gold, 0.4)*acc*ms.z*0.12;
  }
  return light;
}
