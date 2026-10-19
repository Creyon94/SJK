// Star Whale: a thin aqua blade with a pod of whales made of stars swimming round it: a big humpback
// and two calves on their own wider, faster circles, bodies of deep-blue nebula packed with stars,
// starlit outlines, fins sweeping, flukes beating. Each trails rising bubbles; spray and mist droplets
// come off the blade; once a lap the big one blows a fountain of stars. A swing throws an arc of
// water off the blade; a clash makes the big whale breach outward in a splash of droplets.
// @room 8
// @palette {"corona.rim_cool":[0.04,0.30,0.45],"corona.rim_hot":[0.25,0.85,0.95],"corona.inner":[0.75,1.0,1.0],"core.white":[0.92,1.0,1.0],"core.fringe_cool":[0.20,0.70,0.95],"core.fringe_hot":[0.70,1.0,1.0],"light.color":[0.30,0.85,0.95],"trail":[0.35,0.88,0.95]}

const float SW_R = 3.6, SW_T = 0.85, SW_PER = 12.0;
float sw_a(float dt){ return (uTime + dt - 5.9)*6.28318/SW_PER; }
float sw_ai(float dt, float per, float ph){ return (uTime + dt - 5.9 + ph)*6.28318/per; }
float sw_yc(){ return uLength*0.52 + uLength*0.16*sin(uTime*0.09 + 1.0); }
// projected position (xy) and depth (z) on the orbit at angle a
vec3 sw_pos(float a){ return vec3(SW_R*sin(a), sw_yc() - SW_R*cos(a)*sin(SW_T), SW_R*cos(a)*cos(SW_T)); }
vec3 sw_posi(float a, float R, float yo){ return vec3(R*sin(a), sw_yc() + yo - R*cos(a)*sin(SW_T), R*cos(a)*cos(SW_T)); }
float sw_spout(){ return mod(uTime - 5.0, SW_PER); }
// the whale's shape in its own frame (u forward, v toward its back), about 6 long: signed distance
float sw_shape(vec2 q, out float fin, out float fluke){
  float t = uTime;
  // the body wave: the tail stock and flukes beat up and down
  float tw = max(-q.x - 0.6, 0.0);
  q.y -= sin(t*1.6)*0.10*tw*tw;
  float body = sdEll(q - vec2(0.55, 0.0), vec2(2.25, 0.68));
  // flatten the belly behind the jaw, a stout head
  float stock = sdSeg(q, vec2(-0.6, 0.05), vec2(-2.65, 0.12)) - mix(0.42, 0.10, clamp((-q.x - 0.6)/2.0, 0.0, 1.0));
  float k = 0.35;
  float hb = clamp(0.5 + 0.5*(stock - body)/k, 0.0, 1.0);
  float d = mix(stock, body, hb) - k*hb*(1.0 - hb);
  // the dorsal fin: a small hooked bump
  vec2 dq = q - vec2(-1.05, 0.40);
  float dors = sdEll(rot(0.6)*dq, vec2(0.30, 0.13));
  d = min(d, dors);
  // the flukes: two swept lobes at the end of the stock, beating with it
  vec2 fq = q - vec2(-2.75, 0.12);
  fq = rot(sin(t*1.6 - 0.6)*0.35)*fq;
  float l1 = sdEll(rot(0.55)*(fq - vec2(-0.25, 0.32)), vec2(0.48, 0.14));
  float l2 = sdEll(rot(-0.55)*(fq - vec2(-0.25, -0.32)), vec2(0.48, 0.14));
  fluke = min(l1, l2);
  d = min(d, fluke);
  // the long pectoral fin, sweeping slowly
  vec2 pq = q - vec2(1.0, -0.42);
  pq = rot(-2.2 + 0.25*sin(t*0.9))*pq;
  fin = sdEll(pq - vec2(0.85, 0.0), vec2(0.95, 0.15 - 0.05*clamp(pq.x, 0.0, 1.8)));
  return min(d, fin);
}
float sw_star(vec2 v, float s, float px){
  float d = length(v);
  return fillD(d - s*0.35, px) + 0.7*halo(d, s*0.9) + 0.5*(halo(abs(v.x), px + 0.015)*halo(abs(v.y), s*2.0) + halo(abs(v.y), px + 0.015)*halo(abs(v.x), s*2.0))*step(0.15, s);
}
// the whale at b: rgb light, a cover; z its depth
vec4 sw_whale(vec2 b, float px, out float z, float R, float S, float per, float ph, float yo, float big){
  float a = sw_ai(0.0, per, ph);
  R *= 1.0 + 0.35*uClash*big;                            // the breach: the big one leaps outward
  vec3 P = sw_posi(a, R, yo);
  z = P.z;
  vec2 tg = vec2(R*cos(a), R*sin(a)*sin(SW_T));
  float lam = length(tg)/R;
  vec2 f = normalize(tg);
  vec2 up = vec2(-f.y, f.x);
  vec2 C = vec2(0.0, sw_yc() + yo);
  if (dot(up, C - P.xy) < 0.0) up = -up;               // its back toward the blade: a banked turn
  vec2 q = b - P.xy;
  vec2 w = vec2(dot(q, f)/lam, dot(q, up))/S;
  w.y -= 0.06*w.x*w.x;                                  // bent round the turn
  if (abs(w.x) > 4.5 || abs(w.y) > 2.6) return vec4(0.0);
  float fin, fluke;
  float d = sw_shape(w, fin, fluke)*S;
  float inside = fillD(d, px);
  float t = uTime;
  // nebula fill: deep blue back, paler teal belly
  vec3 neb = mix(vec3(0.20, 0.60, 0.85), vec3(0.08, 0.16, 0.55), smoothstep(-0.6, 0.6, w.y));
  neb *= 0.55 + 0.45*fbm3(w*1.8 + vec2(t*0.2, 0.0));
  // packed with tiny stars
  vec2 gp = w*7.0; vec2 gc = floor(gp);
  float st = fillD(length(fract(gp) - h22(gc)) - 0.10, px*7.0/S)*step(0.45, h21(gc))*(0.5 + 0.5*sin(t*4.0 + gc.x*3.1 + gc.y*1.7));
  // throat grooves along the belly behind the jaw
  float belly = smoothstep(-0.05, -0.35, w.y)*smoothstep(0.3, 0.9, w.x)*smoothstep(2.6, 2.0, w.x);
  float grooves = halo(abs(fract(w.y*7.0 + w.x*0.6) - 0.5), 0.08)*belly;
  // the mouth line and the eye
  float mouth = halo(sdSeg(w, vec2(2.75, -0.02), vec2(1.45, -0.28)), 0.03 + px);
  vec3 c = neb*inside*0.75 + vec3(0.85, 0.95, 1.0)*st*inside*1.2 + vec3(0.6, 0.9, 1.0)*grooves*inside*0.6 + vec3(0.7, 0.95, 1.0)*mouth*0.8;
  // the starlit outline and the bright fin edges
  c += vec3(0.65, 0.92, 1.0)*halo(abs(d), 0.035 + px)*1.5 + vec3(0.4, 0.8, 1.0)*halo(max(d, 0.0), 0.22)*0.45;
  // the key stars: snout, eye, dorsal, fin tip, flukes, blowhole
  float ks = sw_star(w - vec2(2.80, 0.05), 0.30, px/S) + sw_star(w - vec2(1.95, 0.10), 0.22, px/S)
           + sw_star(w - vec2(-1.25, 0.55), 0.20, px/S) + sw_star(w - vec2(-3.55, 0.75), 0.22, px/S)
           + sw_star(w - vec2(-3.55, -0.55), 0.22, px/S) + sw_star(w - vec2(1.25, 0.66), 0.16, px/S);
  vec2 ft = vec2(1.0, -0.42) + rot(2.2 - 0.25*sin(t*0.9))*vec2(1.75, 0.0);
  ks += sw_star(w - ft, 0.22, px/S);
  c += vec3(1.0, 0.98, 0.92)*ks*1.3;
  // the spout: a fountain of stars from the blowhole once a lap
  float sp = sw_spout();
  if (sp < 2.2 && big > 0.5){
    for (int i = 0; i < 16; i++){
      float fi = float(i);
      float age = sp - h11(fi*1.7)*0.7;
      if (age < 0.0 || age > 1.5) continue;
      vec2 v0 = vec2((h11(fi*3.1) - 0.5)*1.4, 2.4 + 1.2*h11(fi*5.3));
      vec2 sp2 = vec2(1.25, 0.66) + v0*age + vec2(0.0, -2.2*age*age);
      float k = 1.0 - age/1.5;
      c += vec3(0.85, 0.97, 1.0)*(fillD(length(w - sp2) - 0.06*k - 0.02, px/S) + halo(length(w - sp2), 0.15)*0.6)*k*1.6;
    }
  }
  float lit = mix(0.42, 1.0, smoothstep(-0.5, 0.5, z));
  return vec4(c*lit, inside);
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float L = uLength;
  float z; vec4 wh = sw_whale(b, px, z, SW_R, 1.3, SW_PER, 0.0, 0.0, 1.0);
  float z2, z3;
  vec4 c1 = sw_whale(b, px, z2, 4.6, 0.78, 8.5, 3.0, 7.0, 0.0);
  vec4 c2 = sw_whale(b, px, z3, 5.0, 0.7, 7.0, 6.5, -8.0, 0.0);
  // the blade: thin aqua light with a few stars drifting inside
  float bd = length(vec2(b.x, max(b.y - L, 0.0) + max(-b.y, 0.0)*4.0));
  vec3 blade = vec3(0.25, 0.85, 0.95)*(halo(bd, 0.8)*0.75 + halo(bd, 2.0)*0.12)*smoothstep(-0.3, 0.2, b.y);
  vec2 gp = vec2(b.x*2.5, b.y*1.2 - uTime*0.4); vec2 gc = floor(gp);
  blade += vec3(0.85, 1.0, 1.0)*fillD(length(fract(gp) - h22(gc)) - 0.05, px*2.5)*step(0.82, h21(gc))*(1.0 - smoothstep(0.5, 1.0, abs(b.x)))*step(0.0, b.y)*step(b.y, L);
  // the stardust wake left along its orbit
  vec3 wake = vec3(0.0);
  for (int i = 1; i < 12; i++){
    float fi = float(i);
    vec3 p = sw_pos(sw_a(-2.0 - fi*0.18));
    vec2 jit = (h22(vec2(fi, floor(uTime*3.0))) - 0.5)*0.5;
    float k = 1.0 - fi/12.0;
    wake += vec3(0.7, 0.95, 1.0)*fillD(length(b - p.xy - jit) - 0.05*k - 0.015, px)*k*mix(0.4, 1.0, smoothstep(-0.5, 0.5, p.z))*1.4;
  }
  // bubble trails rising behind each whale: small bright rings that wobble up and pop
  vec3 water = vec3(0.0);
  for (int k = 0; k < 3; k++){
    float fk = float(k);
    float Rk = k == 0 ? SW_R : (k == 1 ? 4.6 : 5.0);
    float pk = k == 0 ? SW_PER : (k == 1 ? 8.5 : 7.0);
    float hk = k == 0 ? 0.0 : (k == 1 ? 3.0 : 6.5);
    float yk = k == 0 ? 0.0 : (k == 1 ? 7.0 : -8.0);
    for (int j = 0; j < 5; j++){
      float fj = float(j);
      float age = fract(uTime*0.6 + fj*0.2 + fk*0.37);
      vec3 p = sw_posi(sw_ai(-age*1.4 - 0.3, pk, hk), Rk, yk);
      vec2 bp = p.xy + vec2(0.25*sin(age*12.0 + fj), age*2.5);
      float rad = 0.08 + 0.12*h11(fj + fk*5.0) + 0.06*age;
      float ring = abs(length(b - bp) - rad);
      water += vec3(0.7, 0.95, 1.0)*(halo(ring, 0.025 + px) + 0.6*halo(length(b - bp - vec2(-0.3, 0.3)*rad), 0.03))*(1.0 - age)*mix(0.45, 1.0, smoothstep(-0.5, 0.5, p.z));
    }
  }
  // spray and mist: droplets flicked off the blade, arcing and fading; more on swing
  for (int i = 0; i < 12; i++){
    float fi = float(i);
    float ph = uTime*(0.6 + 0.4*h11(fi*2.7)) + h11(fi*5.9);
    float cy = floor(ph), f = fract(ph);
    if (h11(cy*3.7 + fi) > 0.5 + 0.5*uSwing) continue;
    float s = h11(cy*1.3 + fi) < 0.5 ? -1.0 : 1.0;
    float y0 = (0.1 + 0.85*h11(cy*2.1 + fi*3.0))*L;
    vec2 v = vec2(s*(2.5 + 3.0*h11(cy + fi*7.0)), 1.0 + 2.0*h11(cy*5.0 + fi));
    vec2 p = vec2(s*0.3, y0) + v*f - vec2(0.0, 2.6*f*f);
    float dd = length(b - p);
    water += vec3(0.75, 0.97, 1.0)*(fillD(dd - 0.07, px) + halo(dd, 0.25)*0.35)*(1.0 - f)*1.3;
    water += vec3(0.35, 0.8, 0.95)*halo(dd, 0.9)*(1.0 - f)*0.06;     // a puff of mist round it
  }
  // swing: an arc of water thrown off the blade
  if (uSwing > 0.02){
    for (int i = 0; i < 18; i++){
      float fi = float(i);
      float u = fi/17.0;
      vec2 p = vec2(1.0 + 4.5*sin(u*3.14159)*uSwing, (0.15 + 0.85*u)*L + 1.5*sin(u*3.14159));
      p += (vec2(h11(fi*3.0), h11(fi*7.0)) - 0.5)*0.6;
      water += vec3(0.75, 0.97, 1.0)*(fillD(length(b - p) - 0.09, px) + halo(length(b - p), 0.3)*0.3)*uSwing*1.2;
    }
  }
  // clash: the breach splash, a crown of droplets thrown up round the big whale
  if (uClash > 0.01){
    vec3 P = sw_posi(sw_ai(0.0, SW_PER, 0.0), SW_R*(1.0 + 0.35*uClash), 0.0);
    float a = 1.0 - uClash;
    for (int i = 0; i < 14; i++){
      float fi = float(i);
      float an = fi*0.449 + 0.3*h11(fi);
      vec2 p = P.xy + vec2(cos(an), sin(an))*(0.8 + 4.5*a) - vec2(0.0, 3.0*a*a);
      water += vec3(0.8, 0.97, 1.0)*(fillD(length(b - p) - 0.1, px) + halo(length(b - p), 0.3)*0.4)*uClash*1.4;
    }
  }
  float front = smoothstep(-0.2, 0.3, z);
  vec3 pod = wh.rgb + c1.rgb + c2.rgb;
  float cover = max(wh.a*front, max(c1.a*smoothstep(-0.2, 0.3, z2), c2.a*smoothstep(-0.2, 0.3, z3)));
  float edge = 1.0 - smoothstep(9.0, 10.5, abs(b.x));
  vec3 outc = (blade*(1.0 - 0.55*cover) + pod + wake + water)*edge*smoothstep(-4.0, -1.0, b.y);
  return (1.0 - exp(-outc*1.1))*1.05;
}
vec3 sig_core(vec3 c, vec2 b, float px){
  float z; vec4 wh = sw_whale(b, px, z, SW_R, 1.3, SW_PER, 0.0, 0.0, 1.0);
  float front = smoothstep(-0.2, 0.3, z);
  return vec3(0.88, 1.0, 1.0)*(1.0 - smoothstep(0.08, 0.35, abs(b.x)))*step(0.0, b.y)*(1.0 - smoothstep(uLength - 0.6, uLength, b.y))*(1.0 - 0.85*wh.a*front);
}
