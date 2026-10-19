// Wraithlight: a pale haunted blade of the Force's spirits. Cold spirit mist pours DOWN from its tip like dry ice and
// spills off at the hilt, and three soul-lights with comet tails climb it in a slow spiral, fade
// out at the tip and are born again at the hilt. A clash scatters the souls and blows the mist
// out in a ring; a swing leaves fading copies of the blade behind it.
// @room 8
// @ghosts {"count":4,"spacing":40.0,"fade":0.5}
// @palette {"corona.rim_cool":[0.30,0.95,0.80],"corona.rim_hot":[0.70,1.0,0.92],"corona.inner":[0.88,1.0,0.97],"core.white":[0.92,1.0,0.97],"core.fringe_cool":[0.35,0.95,0.82],"core.fringe_hot":[0.75,1.0,0.94],"light.color":[0.35,0.95,0.78],"trail":[0.35,0.95,0.78]}

// soft at both ends, so the copies a swing leaves (which have no hilt in front) end cleanly
float wl_on(vec2 b){ return smoothstep(-0.8, 0.6, b.y)*(1.0 - smoothstep(uLength - 0.5, uLength + 1.2, b.y)); }

vec3 wraithlight_glow0(vec3 g, vec2 b, float r, float px){
  float on = wl_on(b);
  float L = uLength;
  float cl = clamp(uClash, 0.0, 1.0);
  float sw = clamp(uSwing, 0.0, 1.0);
  vec3 spirit = vec3(0.35, 0.95, 0.78);
  vec3 pale = vec3(0.85, 1.0, 0.96);
  // --- the blade: a pale, faintly see-through core whose edge wavers like breath on glass
  float wav = 0.06*sin(b.y*1.3 - uTime*3.0) + 0.04*sin(b.y*3.1 + uTime*5.0);
  float d = abs(b.x + wav) + max(b.y - L, 0.0)*0.7;
  float veil = 0.75 + 0.25*vn(vec2(b.x*3.0, b.y*0.6 - uTime*1.5));
  vec3 blade = pale*fillD(d - 0.18, px)*1.1*veil + spirit*halo(d, 0.4)*0.75 + spirit*halo(d, 1.3)*0.15;
  // --- spirit mist pouring down from the tip, thickening toward the hilt and spilling out sideways
  float fall = uTime*(3.0);
  float down = clamp(1.0 - b.y/L, 0.0, 1.0);                       // 0 at the tip, 1 at the hilt
  float spread = 0.75 + 1.1*down*down + 0.9*cl;                     // it widens as it falls
  vec2 q = vec2(b.x/spread*1.4, b.y*0.28 + fall*0.3);
  float n = fbm(q + vec2(fbm3(q*0.8 - uTime*0.15)*1.2, 0.0));
  float curl = (1.0 - smoothstep(spread*0.35, spread, abs(b.x)))*(1.0 - smoothstep(1.8, 2.4, abs(b.x)));
  float mist = smoothstep(0.34, 0.7, n)*curl*(0.6 + 0.4*down);
  // little curls breaking off at the hilt end and drifting out
  float puff = smoothstep(0.55, 0.9, fbm3(vec2(b.x*0.8 - sign(b.x)*uTime*0.6, b.y*0.5 + uTime*0.4)))
             *(1.0 - smoothstep(1.8, 2.4, abs(b.x)))*smoothstep(8.0, 0.0, b.y)*0.6;
  vec3 fog = mix(spirit, pale, 0.25)*(mist*1.5 + puff*0.7);
  // --- three soul-lights climbing the blade in a spiral, each with a fading tail
  vec3 souls = vec3(0.0);
  for (int i = 0; i < 3; i++){
    float fi = float(i);
    float per = 6.0 + fi*1.3;
    float ph = fract(uTime/per + fi/3.0);
    float yc = ph*(L - 2.0) + 1.0;
    float ang = ph*12.566 + fi*2.094 + uTime*0.8;
    float rad = (1.15 + 0.15*sin(uTime*1.7 + fi)) * (1.0 + 0.6*cl);  // a clash flings them out
    float depth = 0.55 + 0.45*cos(ang);                                // behind the blade: dimmer
    float life = smoothstep(0.0, 0.08, ph)*(1.0 - smoothstep(0.85, 1.0, ph));
    vec2 c = vec2(sin(ang)*rad, yc);
    float od = length(b - c);
    float orb = fillD(od - 0.26, px)*1.6 + halo(od, 0.5)*1.3 + halo(od, 1.0)*0.4;
    // the tail: the path it came along, fading over the last stretch
    float tail = 0.0;
    for (int k = 1; k < 7; k++){
      float fk = float(k);
      float pp = ph - fk*0.016;
      vec2 tc = vec2(sin(pp*12.566 + fi*2.094 + (uTime - fk*0.07)*0.8)*rad, pp*(L - 2.0) + 1.0);
      tail += halo(length(b - tc), 0.34 - fk*0.03)*(1.0 - fk/7.0)*0.8;
    }
    souls += mix(spirit, pale, 0.5)*(orb + tail)*depth*life*1.4;
  }
  // --- a clash: a ring of mist bursting outward from the blade
  float ring = halo(abs(abs(b.x) - (1.0 - cl)*2.0), 0.35)*cl*step(0.0, b.y)*step(b.y, L)*0.6;
  vec3 light = (blade + fog + spirit*ring)*on + souls;
  return light*clamp(uIgn*1.4, 0.0, 1.0);
}
vec3 sig_core(vec3 c, vec2 b, float px){ return vec3(0.0); }

// ==== show (added) ====
// Wraithlight show: ghostly spirits drift up and away from the blade, each a wispy hooded shape with
// hollow glowing eyes and a trailing tail; pale green spirit flames lick off its sides; cold fog
// rolls round its base. A swing tears spirits loose; a clash makes them shriek outward in a ring.
vec3 wraithlight_show(vec2 b, float px){
  float L = uLength, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  vec3 spirit = vec3(0.45, 1.0, 0.8), pale = vec3(0.85, 1.0, 0.95);
  vec3 light = vec3(0.0);
  for (int i = 0; i < 6; i++){
    float fi = float(i);
    float ph = uTime*(0.15 + 0.06*h11(fi*2.3)) + h11(fi*7.1);
    float cy = floor(ph), f = fract(ph);
    float s = h11(cy*3.3 + fi) < 0.5 ? -1.0 : 1.0;
    vec2 p = vec2(s*(1.0 + 5.0*f + 0.6*sin(f*8.0 + fi)), (0.1 + 0.6*h11(cy*1.7 + fi))*L + 6.0*f);
    vec2 q = (b - p)/1.2;
    q.x += 0.25*sin(q.y*3.0 - uTime*4.0)*smoothstep(0.0, -1.5, q.y);
    float head = length(q - vec2(0.0, 0.3)) - 0.45;
    float body = length(vec2(q.x*(1.0 + 0.5*max(-q.y, 0.0)), max(-q.y - 0.0, 0.0)*0.35 + min(q.y, 0.0))) - 0.45 + max(-q.y, 0.0)*0.18;
    float g = min(head, max(body, q.y - 0.3));
    g = max(g, -2.5 - q.y);
    float eyes = fillD(length(vec2(abs(q.x) - 0.17, q.y - 0.35)*vec2(1.0, 0.7)) - 0.09, px*2.0);
    float mouth = fillD(length((q - vec2(0.0, 0.08))*vec2(1.6, 0.8)) - 0.09, px*2.0);
    float fade = smoothstep(0.0, 0.15, f)*(1.0 - smoothstep(0.7, 1.0, f));
    float core = fillD(g*1.2, px)*0.35;
    light += (spirit*(core + exp(-pow(max(g, 0.0)/0.05, 2.0))*0.8 + halo(max(g, 0.0), 0.35)*0.25) - spirit*(eyes + mouth)*0.4 + pale*(eyes + mouth)*0.0)*fade;
    light += vec3(0.0)*eyes;
  }
  for (int i = 0; i < 6; i++){
    float fi = float(i);
    float ph = uTime*(0.55 + 0.3*h11(fi*2.7)) + h11(fi*6.1);
    float cy = floor(ph), f = fract(ph);
    float s = h11(cy*3.1 + fi) < 0.5 ? -1.0 : 1.0;
    float y0 = (0.1 + 0.8*h11(cy*1.9 + fi))*L;
    float u = (b.x*s - 0.4)/2.0;
    if (u < 0.0 || u > 1.0) continue;
    float dy = b.y - y0 - 1.6*u*u - 0.35*sin(u*7.0 - uTime*10.0)*u;
    light += mix(pale, spirit, u)*exp(-pow(dy/(0.22*(1.0 - u) + 0.03), 2.0))*(1.0 - u)*(1.0 - f)*smoothstep(0.0, 0.1, f)*1.2;
  }
  float fog = fbm(vec2(b.x*0.3 + uTime*0.15, b.y*0.25 - uTime*0.05));
  light += spirit*smoothstep(0.45, 0.8, fog)*exp(-max(b.y, 0.0)/5.0)*smoothstep(0.8, 2.0, abs(b.x))*(1.0 - smoothstep(5.0, 8.0, abs(b.x)))*0.4;
  if (cl > 0.01){
    vec2 o = vec2(0.0, L*0.6);
    light += pale*halo(abs(length(b - o) - 1.0 - 7.0*(1.0 - cl)), 0.15)*cl*1.5;
  }
  float edge = 1.0 - smoothstep(8.5, 10.2, abs(b.x));
  light *= edge*(1.0 - smoothstep(L + 0.5, L + 3.5, b.y))*smoothstep(-3.0, -0.5, b.y);
  return (1.0 - exp(-light*1.15))*1.05;
}

// ==== the spirits (round 7) ====
// Force spirits round the blade: cold spectral flames lick up both its edges, three souls spiral
// round it on long glowing tails reaching four units out, and spirit flames break off the edges
// and rise away. A swing whips the souls out wide; a clash flashes every spirit at once.
vec3 wl_spirits(vec2 b, float px){
  float L = uLength, t = uTime, ax = abs(b.x);
  float sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  vec3 spirit = vec3(0.45, 1.0, 0.85), pale = vec3(0.85, 1.0, 0.97);
  float along = smoothstep(-0.5, 1.5, b.y)*(1.0 - smoothstep(L - 1.0, L + 1.5, b.y));
  vec3 light = vec3(0.0);
  // spectral flames up both edges
  float fl = fbm3(vec2(ax*1.4 - t*0.3, b.y*0.45 - t*2.2) + vec2(step(0.0, b.x)*7.0, 0.0));
  float flame = smoothstep(0.35, 0.9, fl*1.4 - (ax - 0.4)*0.45)*smoothstep(0.25, 0.7, ax);
  light += spirit*flame*0.9*along + pale*flame*flame*0.6*along;
  // three souls spiralling round on long tails
  for (int k = 0; k < 3; k++){
    float fk = float(k);
    float y = fract(t*0.09 + fk/3.0)*(L + 4.0) - 2.0;
    float R = 3.0 + 1.5*sw + 0.5*sin(t + fk);
    for (int j = 0; j < 16; j++){
      float fj = float(j);
      float yy = y - fj*0.45;
      vec2 h = helix(yy, R*(1.0 - fj*0.03), 0.8, t*0.25 + fk/3.0);
      float d = length(b - vec2(h.x, yy));
      float w = j == 0 ? 0.55 : 0.38 - fj*0.02;
      light += mix(pale, spirit, fj/16.0)*halo(d, max(w, 0.05))*(j == 0 ? 2.6 : 0.55*(1.0 - fj/16.0))*(0.45 + 0.55*smoothstep(-0.6, 0.6, h.y));
    }
  }
  // spirit flames breaking off the edges and rising away, curling as they fade
  for (int i = 0; i < 10; i++){
    float fi = float(i);
    float ph = t*(0.35 + 0.2*h11(fi*2.9)) + h11(fi*6.3);
    float cyc = floor(ph), f = fract(ph);
    float sd = h11(cyc + fi*3.0) < 0.5 ? -1.0 : 1.0;
    vec2 p = vec2(sd*(0.8 + 5.0*f + 0.9*sin(f*7.0 + fi)), (0.08 + 0.8*h11(cyc*1.7 + fi))*L + 6.0*f);
    vec2 q = b - p;
    float tongue = length(vec2(q.x*1.4, max(q.y, 0.0)*0.35 + min(q.y, 0.0)*1.3)) - 0.55*(1.0 - f) - 0.1*sin(t*12.0 + fi + q.y*5.0);
    light += (pale*fillD(tongue + 0.25, px)*0.9 + spirit*halo(max(tongue, 0.0), 0.35)*0.8)*(1.0 - f)*smoothstep(0.0, 0.1, f)*(1.0 + cl);
  }
  float edge = 1.0 - smoothstep(8.5, 10.0, ax);
  light *= edge*(1.0 - smoothstep(L + 1.0, L + 4.0, b.y))*smoothstep(-2.5, -0.5, b.y);
  return (1.0 - exp(-max(light, vec3(0.0))*1.15))*1.05;
}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  px = min(px, 0.06);
  return wraithlight_glow0(g, b, r, px) + wraithlight_show(b, px) + wl_spirits(b, px);
}
