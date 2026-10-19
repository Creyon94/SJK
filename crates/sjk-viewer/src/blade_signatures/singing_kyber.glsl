// Singing Kyber: a kyber crystal that sings, as kyber does. The blade is a long six-sided
// crystal of teal light with a white-hot heart: its faces and edges catch the light and glint
// as you turn it, and fracture planes deep inside it flash. It holds a standing note: bright
// bands sit still along its length and tremble, and between them a shimmering harmonic aura
// swells and shrinks with the wave. Sound rings leave the bands and travel up the blade,
// spreading outward and fading. A swing raises the pitch (more bands, faster rings, a brighter
// aura); a clash is a sonic boom: a shockwave sphere bursts out from the blade.
// @room 8
// @palette {"corona.rim_cool":[0.11,0.50,0.47],"corona.rim_hot":[0.2,0.9,0.85],"corona.inner":[0.9,1.0,1.0],"core.white":[1.0,1.0,1.0],"core.fringe_cool":[0.12,0.54,0.51],"core.fringe_hot":[0.2,0.9,0.85],"light.color":[0.2,0.9,0.85],"trail":[0.2,0.9,0.85]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){ return vec3(0.0); }
vec3 sig_core(vec3 c, vec2 b, float px){ return vec3(0.0); }

// clip the span (x..y) by the half-space dot(p, n) <= c; z, w: the entry normal's angle code
vec4 sk_cut(vec3 ro, vec3 rd, vec3 n, float c, vec4 sp, float code){
  float dn = dot(rd, n), o = dot(ro, n);
  if (abs(dn) < 1e-6){
    if (o > c) sp.y = -1.0;
    return sp;
  }
  float tc = (c - o)/dn;
  if (dn > 0.0){ sp.y = min(sp.y, tc); }
  else if (tc > sp.x){ sp.x = tc; sp.z = code; }
  return sp;
}
// a flattened shell of sound: the ray's closest distance to centre c with heights squeezed by k
// (so the shell is a lens k times wider than tall), and the light of a shell of radius R there
float sk_shell(vec3 ro, vec3 rd, vec3 c, float R, float k, float w){
  vec3 o = ro - c; o.y *= k;
  vec3 d = rd; d.y *= k; d = normalize(d);
  float tt = max(dot(-o, d), 0.0);
  float r = length(o + d*tt);
  float inside = clamp(w/sqrt(max(R*R - r*r, w*w)), 0.0, 1.0)*smoothstep(R + w, R - 2.0*w, r);
  return inside + halo(abs(r - R), w*1.5)*0.35;
}
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime;
  float sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  vec3 teal = vec3(0.2, 0.92, 0.86), pale = vec3(0.85, 1.0, 1.0), white = vec3(1.0);
  float top = L*clamp(uAge/0.45, 0.0, 1.0);
  float fx = uFx;
  // the standing wave: nodes (bright bands) along the blade
  float N = 7.0 + 2.0*sw;
  float vib = 0.75 + 0.25*sin(t*41.0) + 0.15*sin(t*67.0);
  // ---- the crystal: a hexagonal prism of light with a pointed tip
  float ap = 0.42;
  float hc = 1.6;
  vec4 sp = vec4(0.0, 1e5, 0.0, 0.0);
  sp = sk_cut(ro, rd, vec3(0.0, -1.0, 0.0), 0.6, sp, 7.0);
  for (int k = 0; k < 6; k++){
    float a = (float(k) + 0.5)*1.0472;
    vec3 n = vec3(cos(a), 0.0, sin(a));
    sp = sk_cut(ro, rd, n, ap, sp, float(k));
    vec3 m = vec3(cos(a)*hc, ap, sin(a)*hc);
    sp = sk_cut(ro, rd, m, top*ap, sp, float(k) + 10.0);
  }
  vec3 light = vec3(0.0);
  float s;
  float dax = raySeg(ro, rd, vec3(0.0, -0.6, 0.0), vec3(0.0, top - 0.6, 0.0), s);
  float yax = mix(-0.6, top - 0.6, s);
  float node = pow(abs(cos(3.14159*N*clamp(yax, 0.0, L)/L)), 6.0);
  // the white heart of the crystal and its glow (fills the emitter)
  light += white*exp(-pow(dax/max(0.2, px), 2.0))*1.3 + teal*halo(dax, 0.9)*(0.5 + 0.35*node*vib) + teal*halo(dax, 2.6)*0.13;
  if (sp.y > sp.x){
    vec3 P = ro + rd*sp.x;
    vec3 Q = ro + rd*sp.y;
    float th = sp.y - sp.x;
    float code = sp.z;
    // which face: its angle; the lateral position across it, to find the edges
    float fa = (mod(code, 10.0) + 0.5)*1.0472;
    vec2 tg = vec2(-sin(fa), cos(fa));
    float lat = abs(dot(P.xz, tg));
    float edge = code < 6.5 ? ap*0.57735 - lat : 1.0;
    float tipEdge = code > 9.5 ? ap*0.57735 - lat : 1.0;
    // the face's glint: a light sweeping round the blade, catching each face in turn
    float sweep = 0.5 + 0.5*cos(fa - t*1.7 - P.y*0.12);
    float glint = pow(sweep, 18.0);
    // internal fracture planes flashing deep inside
    vec3 M = (P + Q)*0.5;
    float fr1 = pow(0.5 + 0.5*sin(dot(M, vec3(0.8, 0.45, 0.4))*5.0 + t*0.8), 30.0);
    float fr2 = pow(0.5 + 0.5*sin(dot(M, vec3(-0.5, 0.6, 0.62))*4.3 - t*0.6), 30.0);
    float spark = step(0.86, h21(vec2(floor(M.y*1.3), floor(t*6.0 + mod(code, 10.0)*0.37))));
    float lit = smoothstep(-0.6, 0.2, P.y);
    float gate = smoothstep(0.45, 0.8, vn(vec2(M.y*0.45, t*1.3)));
    vec3 c = teal*(0.25 + 0.6*th) + pale*(fr1 + fr2)*min(th, 0.8)*1.3*gate
           + white*halo(min(edge, tipEdge), max(px*0.7, 0.025))*(0.45 + 1.4*glint + 0.8*spark)
           + pale*glint*0.5 + pale*node*vib*0.8*min(th*1.5, 1.0);
    light += c*lit*(1.0 + 0.4*cl);
  }
  if (fx > 0.0){
    vec3 fxl = vec3(0.0);
    float near = raySeg(ro, rd, vec3(0.0, 0.0, 0.0), vec3(0.0, top, 0.0), s);
    if (near < 6.5){
      // ---- the standing-wave nodes: bright bands that tremble, flat glows round the crystal
      for (int i = 0; i < 9; i++){
        float fi = float(i);
        if (fi >= N) continue;
        float y = (fi + 0.5)*L/N;
        if (y > top) continue;
        float sh = sk_shell(ro, rd, vec3(0.0, y, 0.0), 0.95 + 0.15*sin(t*41.0 + fi*1.7), 4.0, 0.14);
        float to;
        float dn = rayPt(ro, rd, vec3(0.0, y, 0.0), to);
        fxl += (pale*sh*0.8 + teal*halo(dn, 1.3)*0.5)*vib;
      }
      // ---- sound rings travelling up the blade and spreading outward
      for (int i = 0; i < 8; i++){
        float fi = float(i);
        float ph = t*(0.55 + 0.25*sw) + fi/8.0;
        float f = fract(ph);
        float cyc = floor(ph);
        float k = floor(h11(cyc*1.7 + fi*3.1)*N);
        float y0 = (k + 0.5)*L/N;
        float y = y0 + f*7.0;
        float R = 0.8 + f*5.0;
        if (y > top + 1.0) continue;
        float w = 0.12 + 0.35*f;
        float amp = (1.0 - f)*(1.0 - f)*smoothstep(0.0, 0.1, f)*(1.0 + 0.6*sw);
        float sh = sk_shell(ro, rd, vec3(0.0, y, 0.0), R, 2.2, w);
        fxl += mix(pale, teal, 0.3 + 0.5*f)*sh*amp*1.1;
      }
      // ---- the harmonic aura: a shimmering glow that swells at the wave's peaks
      vec3 m = marchSteps(ro, rd, 3.2, -0.3, top + 0.8, 16.0);
      if (m.y > m.x){
        float tt = m.x + m.z*h21(rd.xy*517.0 + ro.xz*3.7);
        float acc = 0.0;
        for (int k = 0; k < 16; k++){
          vec3 p = ro + rd*tt;
          float r = length(p.xz);
          float anti = abs(sin(3.14159*N*clamp(p.y, 0.0, L)/L));
          float Ra = 0.7 + 1.5*anti*(0.8 + 0.2*sin(t*23.0));
          vec2 q = rot(t*0.7) * p.xz;
          float sh = fbm3(vec2(q.x*1.5 + p.y*0.2, q.y*1.5 - t*2.0));
          acc += exp(-pow(r/Ra, 2.0))*(0.35 + 0.9*sh*sh)*smoothstep(-0.3, 1.0, p.y)*(1.0 - smoothstep(top - 1.0, top + 0.8, p.y))*m.z;
          tt += m.z;
        }
        fxl += mix(teal, pale, 0.2)*acc*(0.15 + 0.1*sw);
      }
    }
    // ---- a clash: the sonic boom, a shockwave sphere bursting out
    if (cl > 0.01){
      vec3 o = vec3(0.0, L*0.6, 0.0);
      float to;
      float dc = rayPt(ro, rd, o, to);
      float R = 1.8 + 5.0*(1.0 - cl);
      float sw2 = 0.35 + 0.5*(1.0 - cl);
      float shell = sk_shell(ro, rd, o, R, 1.3, sw2);
      fxl += (pale*shell*1.1 + teal*shell*0.6)*cl + pale*halo(dc, 3.0)*pow(cl, 3.0)*0.35;
    }
    light += fxl*fx;
  }
  return (1.0 - exp(-light*1.1))*1.05;
}
