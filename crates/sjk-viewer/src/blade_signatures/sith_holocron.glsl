// Sith Holocron: the blade is a holocron's crimson crystal stretched into a blade: a real
// three-sided crystal prism with a slow twist, its glass cut into triangular facets that glint
// as you turn it, coming to a point at the tip. A red heart beats deep inside it, and Sith
// knowledge streams up through the glass as thin lines of light, facet after facet waking as the
// stream passes. Every ten seconds the holocron opens: the crystal splits along a seam, the halves
// part on a white-hot core, and red smoke and lightning pour out of the crack all round before it
// seals again. A swing makes the facets flare and lightning leap off the glass; a clash throws out
// a triangular shockwave; on ignition the crystal grows from the hilt, facets waking as it rises.
// @room 8
// @solid 1
// @palette {"corona.rim_cool":[0.40,0.0,0.02],"corona.rim_hot":[1.0,0.10,0.06],"corona.inner":[1.0,0.65,0.60],"core.white":[1.0,0.92,0.90],"core.fringe_cool":[0.85,0.03,0.04],"core.fringe_hot":[1.0,0.30,0.22],"light.color":[1.0,0.12,0.08],"trail":[1.0,0.12,0.08]}

const float HC_A = 0.45;       // the crystal's inner radius (its faces' distance from the axis)
float hc_top(){ return uLength*clamp(uAge/0.45, 0.0, 1.0); }
// how far the holocron is open: 0 sealed .. 1 split wide
float hc_open(){
  float c = mod(uTime - 2.0, 10.0);
  return smoothstep(0.0, 0.9, c)*(1.0 - smoothstep(3.2, 4.2, c));
}
// the seam's half gap at height y (widest in the middle of the blade)
float hc_gap(float y){
  float u = clamp(y/uLength, 0.0, 1.0);
  return 0.30*hc_open()*pow(max(sin(3.1416*u), 0.0), 0.6);
}
float hc_beat(){ return 0.55 + 0.3*pow(0.5 + 0.5*sin(uTime*2.4), 6.0) + 0.25*pow(0.5 + 0.5*sin(uTime*2.4 - 0.7), 10.0); }
// the crystal's cross-section turned with its twist at height y
vec2 hc_q(vec3 p){ return rot(0.35 + p.y*0.045)*p.xz; }
// the crystal's inner radius at y: a point at the tip, a short neck at the hilt
float hc_a(float y){
  float L = uLength, top = hc_top();
  return HC_A*(1.0 - smoothstep(top - 2.6, top + 0.2, y)*0.97)*smoothstep(-0.2, 0.5, y);
}
float hc_prism(vec2 q, float a){
  // a smooth maximum of the three faces: crisp facets with softened corners
  float d0 = q.y, d1 = dot(q, vec2(0.866, -0.5)), d2 = dot(q, vec2(-0.866, -0.5));
  float k = 0.15*a + 0.002;
  float m = max(max(d0, d1), d2);
  return m + k*log(exp((d0 - m)/k) + exp((d1 - m)/k) + exp((d2 - m)/k)) - a;
}
float hc_sdf(vec3 p){
  float g = hc_gap(p.y);
  float s = p.x >= 0.0 ? 1.0 : -1.0;
  vec3 c = vec3(p.x - s*g, p.y, p.z);
  float d = hc_prism(hc_q(c), hc_a(p.y));
  d = max(d, -s*c.x - 0.0001*step(g, 0.001));        // each half ends at the seam
  d = max(d, max(0.1 - p.y, p.y - hc_top() - 0.3));
  return d;
}
// a triangle lattice: x = distance to the nearest facet edge, yz = the facet's id
vec3 hc_tri(vec2 p){
  vec2 s = vec2(p.x + p.y*0.57735, p.y*1.1547);
  vec2 i = floor(s), f = fract(s);
  float up = step(f.x + f.y, 1.0);
  float e = up > 0.5 ? min(min(f.x, f.y), 1.0 - f.x - f.y) : min(min(1.0 - f.x, 1.0 - f.y), f.x + f.y - 1.0);
  return vec3(e*0.866, i + vec2(up*0.5, 0.0));
}
// the knowledge stream: height of the waking front
float hc_front(){ return mod(uTime*9.0, uLength + 16.0) - 4.0; }
// red smoke: density at p (two crossed noise slices, so it is round in 3D)
float hc_dens(vec3 p){
  float t = uTime, L = uLength;
  float r = length(p.xz);
  float op = hc_open();
  float n = fbm3(vec2(p.x*0.6 - sign(p.x)*t*0.6*op + 2.0, p.y*0.3 - t*0.8 - r*0.3))*fbm3(vec2(p.z*0.6 + 5.0, p.y*0.27 - t*0.6 + r*0.2));
  n = smoothstep(0.15, 0.6, n);
  // a thin veil hugging the glass, pouring out wide from the crack while open
  float veil = exp(-max(r - 0.7, 0.0)/(0.8 + 1.8*op))*smoothstep(0.35, 0.8, r)*(0.5 + 1.6*op*smoothstep(0.0, 0.25, hc_gap(p.y)));
  float cl = uClash > 0.01 ? exp(-length(p - vec3(0.0, L*0.6, 0.0))/2.0)*uClash*1.5 : 0.0;
  return n*(veil + cl)*(1.0 - smoothstep(3.0, 4.8, r))*smoothstep(-0.5, 1.0, p.y)*smoothstep(hc_top() + 2.0, hc_top() - 0.5, p.y);
}
vec3 hc_smoke(vec3 ro, vec3 rd, float tmax){
  vec3 m = marchSteps(ro, rd, 4.8, -0.5, uLength + 2.0, 16.0);
  float te = min(m.y, tmax);
  if (te <= m.x) return vec3(0.0);
  float dt = (te - m.x)/16.0;
  float t = m.x + dt*h21(rd.xy*517.0 + ro.xz);
  float lit = 0.0, acc = 0.0;
  for (int i = 0; i < 16; i++){
    vec3 p = ro + rd*t;
    float d = hc_dens(p);
    if (d > 0.001){
      float r = length(p.xz);
      lit += d*(0.3 + 1.2*exp(-r/1.0))*exp(-acc*0.8)*dt;
      acc += d*dt;
    }
    t += dt;
  }
  return vec3(0.75, 0.05, 0.03)*lit*1.7;
}
// lightning: crawling off the glass and leaping out of the crack. front: 1 = only the bolts on
// the viewer's side of the crystal, 0 = all of them
vec3 hc_bolts(vec3 ro, vec3 rd, float px, float front){
  float L = uLength, t = uTime;
  float act = max(hc_open(), clamp(uSwing, 0.0, 1.0));
  vec3 light = vec3(0.0);
  vec2 toCam = -normalize(rd.xz + vec2(1e-5));
  for (int i = 0; i < 5; i++){
    float fi = float(i);
    float k = floor(t*10.0 + fi*0.37);
    float live = step(0.5 - 0.4*act, h11(k*1.3 + fi*7.0));
    if (live < 0.5) continue;
    float y0 = (0.08 + 0.84*h11(k*0.7 + fi*3.1))*hc_top();
    float an = h11(k + fi*2.0);
    vec3 a = around(HC_A*0.9 + hc_gap(y0), an, y0);
    vec3 c = around(HC_A + 1.8 + 3.0*act*h11(k*2.3 + fi), an + 0.12*(h11(k*5.0 + fi) - 0.5), y0 + (h11(k*3.7 + fi) - 0.35)*4.0);
    float facing = step(0.0, dot(c.xz, toCam));
    if (front > 0.5 && facing < 0.5) continue;
    float d = bolt3(ro, rd, a, c, k + fi*11.0, 0.38);
    light += (boltLight(d, vec3(1.0, 0.1, 0.05), px)*0.5 + vec3(1.0, 0.1, 0.05)*halo(d, 0.35)*0.5)*(0.4 + 0.9*act);
  }
  return light;
}
vec4 sig_vol_solid(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime;
  float top = hc_top();
  vec4 col = vec4(0.0);
  if (top < 0.3) return col;
  vec2 sp = raySpan(ro, rd, HC_A*2.2 + 0.35, 0.0, top + 0.4);
  if (sp.y <= sp.x) return col;
  float tt = sp.x, dmin = 1e3, tmin = sp.x;
  float hit = 0.0;
  for (int i = 0; i < 40; i++){
    float d = hc_sdf(ro + rd*tt);
    if (d < dmin){ dmin = d; tmin = tt; }
    if (d < 0.002){ hit = 1.0; break; }
    tt += max(d*0.8, 0.004);
    if (tt > sp.y) break;
  }
  float cov = hit > 0.5 ? 1.0 : clamp(1.0 - dmin/max(px, 1e-3), 0.0, 1.0);
  if (cov <= 0.0) return col;
  float th = hit > 0.5 ? tt : tmin;
  vec3 p = ro + rd*th;
  vec2 e = vec2(0.02, 0.0);
  vec3 n = normalize(vec3(hc_sdf(p + e.xyy) - hc_sdf(p - e.xyy), hc_sdf(p + e.yxy) - hc_sdf(p - e.yxy), hc_sdf(p + e.yyx) - hc_sdf(p - e.yyx)));
  float g = hc_gap(p.y);
  float side = p.x >= 0.0 ? 1.0 : -1.0;
  vec2 q = hc_q(vec3(p.x - side*g, p.y, p.z));
  // which face, and where across it
  float d0 = q.y, d1 = dot(q, vec2(0.866, -0.5)), d2 = dot(q, vec2(-0.866, -0.5));
  float fid = d0 > max(d1, d2) ? 0.0 : (d1 > d2 ? 1.0 : 2.0);
  vec2 tg = fid < 0.5 ? vec2(1.0, 0.0) : (fid < 1.5 ? vec2(-0.5, -0.866) : vec2(-0.5, 0.866));
  float u = dot(q, tg);
  float a = hc_a(p.y);
  // the facets: a triangle lattice cut into each face
  vec3 tr = hc_tri(vec2(u*2.2 + fid*3.7 + 0.3*side, p.y*0.45));
  float id = h21(tr.yz + fid*13.0 + side*5.0);
  float id2 = h21(tr.yz*1.7 + fid*3.0);
  // each facet tilted a little its own way, so they glint one by one as the blade turns
  vec3 tg3 = normalize(vec3(rot(-(0.35 + p.y*0.045))*tg, 0.0).xzy);
  n = normalize(n + tg3*(id - 0.5)*0.55 + vec3(0.0, 1.0, 0.0)*(id2 - 0.5)*0.5);
  float awake = smoothstep(0.0, 0.25, uAge*2.2 - p.y/L - id*0.15);
  float beat = hc_beat();
  // the knowledge stream rising: facets lit as it passes, thin lines of light climbing
  float fr = hc_front();
  float pass = exp(-pow((p.y - fr)/3.0, 2.0)) + 0.6*exp(-pow((p.y - fr + 14.0)/2.0, 2.0));
  float lk = p.y*2.6 - t*3.5 + id*0.5;
  float lines = halo(abs(fract(lk) - 0.5), 0.06)*step(0.55, h11(floor(lk) + fid*5.0 + side*3.0))*halo(abs(fract(u*3.0 + id) - 0.5), 0.12);
  float lit = clamp(0.12 + 0.9*id*id*beat + pass*(0.5 + 0.6*id) + 0.9*uSwing*step(0.5, id), 0.0, 2.0);
  vec3 face = mix(vec3(0.22, 0.0, 0.015), vec3(0.95, 0.10, 0.05), clamp(lit, 0.0, 1.0)*0.75) + vec3(1.0, 0.5, 0.4)*max(lit - 1.0, 0.0)*0.6;
  // the heart: red light deep inside, seen through the glass where the ray passes the axis
  float sAx;
  float dAx = raySeg(ro, rd, vec3(0.0, 0.0, 0.0), vec3(0.0, top, 0.0), sAx);
  face += vec3(1.0, 0.12, 0.06)*halo(dAx, a*0.8 + 0.05)*beat*0.9 + vec3(1.0, 0.55, 0.45)*halo(dAx, 0.12)*beat*0.35;
  face += vec3(1.0, 0.35, 0.25)*lines*0.8*(0.4 + pass);
  // facet edges: bright cut lines in the glass
  face += vec3(1.0, 0.45, 0.35)*halo(tr.x, 0.03 + px*1.2)*(0.45 + 0.8*pass + 0.6*uSwing);
  // glass: a hot rim where it turns away, a sharp glint off the facets
  vec3 v = -rd;
  vec3 kl = normalize(vec3(-0.4, 0.7, 0.6));
  float fres = pow(1.0 - abs(dot(n, v)), 3.0);
  float spec = pow(max(dot(reflect(-kl, n), v), 0.0), 40.0);
  face += vec3(1.0, 0.2, 0.12)*fres*0.7 + vec3(1.0, 0.85, 0.8)*spec*0.9;
  // the seam faces glow white-hot from the core when open
  float seamFace = smoothstep(0.6, 0.95, abs(n.x))*smoothstep(0.02, 0.1, g)*step(abs(p.x), g + 0.12);
  face += vec3(1.0, 0.75, 0.65)*seamFace*1.6*hc_open();
  face *= mix(0.25, 1.0, awake);
  // what hangs in front of the glass: smoke and the bolts on this side
  face += (hc_smoke(ro, rd, th) + hc_bolts(ro, rd, px, 1.0))*uFx;
  return vec4(face, cov*mix(0.3, 0.97, awake));
}
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime;
  float top = hc_top();
  float op = hc_open();
  float beat = hc_beat();
  vec3 red = vec3(1.0, 0.1, 0.05);
  // the core: white-hot inside, seen in the emitter and through the open seam
  vec3 light = blade3(ro, rd, px, red, 0.32 + 0.08*op, top);
  float s;
  float d = raySeg(ro, rd, vec3(0.0, 0.0, 0.0), vec3(0.0, top, 0.0), s);
  // the crystal's red aura beating with the heart, and the light spilling from the crack
  light += red*halo(max(d - HC_A*1.3, 0.0), 0.6)*0.8*beat + vec3(0.6, 0.0, 0.02)*halo(d, 3.0)*0.22;
  light += vec3(1.0, 0.25, 0.12)*halo(d, 0.6 + 1.4*op)*op*0.6*sin(3.1416*clamp(s, 0.0, 1.0));
  if (uFx > 0.01){
    light += (hc_smoke(ro, rd, 1e5) + hc_bolts(ro, rd, px, 0.0))*uFx;
    // clash: a triangular shockwave, the holocron's face, rushing out across the blade
    if (uClash > 0.01){
      float a = 1.0 - uClash;
      float hy = L*0.6;
      vec3 h = rayAt(ro, rd, hy);
      if (h.z > 0.0){
        vec2 q = hc_q(vec3(h.x, hy, h.y));
        float R = 2.5 + 9.0*pow(a, 0.6);
        float tri = max(max(q.y, dot(q, vec2(0.866, -0.5))), dot(q, vec2(-0.866, -0.5))) - R*0.5;
        light += red*(halo(abs(tri), 0.15 + 0.3*a)*1.8 + halo(max(-tri, 0.0), 1.2)*0.35)*uClash*(1.0 - smoothstep(3.0, 6.0, length(h.xy)))*uFx;
      }
      float tc; float dc = rayPt(ro, rd, vec3(0.0, hy, 0.0), tc);
      light += red*halo(dc, 1.3)*pow(uClash, 3.0)*1.5*uFx;
    }
  }
  return (1.0 - exp(-light*1.15))*1.05;
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){ return vec3(0.0); }
vec3 sig_core(vec3 c, vec2 b, float px){ return vec3(0.0); }
