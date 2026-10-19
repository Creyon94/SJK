// Nine Lives: a thin moonlit lavender blade with a gang of glowing cats round it, drawn as bright
// neon-line figures in different poses: one sitting with its tail curled, walkers prowling, a
// leaper mid-pounce, a sleepy curled-up cat drifting by. They circle the blade in front and behind,
// paw prints walk up its edge, and pairs of cat eyes blink open in the dark around it.
// A swing makes the cats pounce outward; a clash bursts out nine cat-spirit heads (nine lives);
// ignition: a cat strolls up the blade leaving prints.
// @room 8
// @palette {"corona.rim_cool":[0.55,0.40,0.95],"corona.rim_hot":[0.95,0.70,1.0],"corona.inner":[1.0,0.95,1.0],"core.white":[1.0,0.98,1.0],"core.fringe_cool":[0.75,0.60,1.0],"core.fringe_hot":[1.0,0.85,1.0],"light.color":[0.80,0.65,1.0],"trail":[0.78,0.62,1.0]}

// a triangle (for ears): distance to the triangle a b c
float ct_tri(vec2 p, vec2 a, vec2 b, vec2 c){
  vec2 e0 = b - a, e1 = c - b, e2 = a - c;
  vec2 v0 = p - a, v1 = p - b, v2 = p - c;
  vec2 pq0 = v0 - e0*clamp(dot(v0, e0)/dot(e0, e0), 0.0, 1.0);
  vec2 pq1 = v1 - e1*clamp(dot(v1, e1)/dot(e1, e1), 0.0, 1.0);
  vec2 pq2 = v2 - e2*clamp(dot(v2, e2)/dot(e2, e2), 0.0, 1.0);
  float s = sign(e0.x*e2.y - e0.y*e2.x);
  vec2 d = min(min(vec2(dot(pq0, pq0), s*(v0.x*e0.y - v0.y*e0.x)),
                   vec2(dot(pq1, pq1), s*(v1.x*e1.y - v1.y*e1.x))),
                   vec2(dot(pq2, pq2), s*(v2.x*e2.y - v2.y*e2.x)));
  return -sqrt(d.x)*sign(d.y);
}
float ct_smin(float a, float b, float k){ float h = clamp(0.5 + 0.5*(b - a)/k, 0.0, 1.0); return mix(b, a, h) - k*h*(1.0 - h); }
// a cat's head at c (radius r) with two ears, facing +x
float ct_head(vec2 q, vec2 c, float r){
  float d = length(q - c) - r;
  d = min(d, ct_tri(q, c + vec2(-0.55, 0.55)*r, c + vec2(-0.05, 0.75)*r, c + vec2(-0.6, 1.45)*r));
  d = min(d, ct_tri(q, c + vec2(0.15, 0.78)*r, c + vec2(0.7, 0.5)*r, c + vec2(0.55, 1.4)*r));
  return d;
}
// the cat figures in their own frame (+x forward, +y up), about 2.4 long; kind 0 sit, 1 walk,
// 2 leap, 3 curled asleep. Returns a signed distance.
float ct_cat(vec2 q, float kind, float t){
  float d;
  if (kind < 0.5){                                    // sitting, tail curled round
    float body = sdEll(q - vec2(-0.05, 0.62), vec2(0.5, 0.72));
    float chest = sdEll(q - vec2(0.22, 0.95), vec2(0.28, 0.42));
    d = ct_smin(body, chest, 0.15);
    d = ct_smin(d, ct_head(q, vec2(0.28, 1.55), 0.36), 0.08);
    float sw = 0.15*sin(t*2.0);
    d = min(d, sdSeg(q, vec2(-0.45, 0.08), vec2(-0.85, 0.25 + sw)) - 0.08);
    d = min(d, sdSeg(q, vec2(-0.85, 0.25 + sw), vec2(-0.75, 0.75 + sw*1.5)) - 0.075);
    d = min(d, sdSeg(q, vec2(0.25, 0.0), vec2(0.3, 0.6)) - 0.09);          // front leg
  } else if (kind < 1.5){                             // walking, legs stepping
    float body = sdEll(q - vec2(0.0, 0.78), vec2(0.95, 0.33));
    d = ct_smin(body, ct_head(q, vec2(1.02, 1.1), 0.32), 0.12);
    float s = sin(t*7.0);
    d = min(d, sdSeg(q, vec2(0.62, 0.62), vec2(0.62 + 0.22*s, 0.0)) - 0.08);
    d = min(d, sdSeg(q, vec2(0.48, 0.62), vec2(0.48 - 0.22*s, 0.0)) - 0.08);
    d = min(d, sdSeg(q, vec2(-0.6, 0.62), vec2(-0.6 - 0.22*s, 0.0)) - 0.09);
    d = min(d, sdSeg(q, vec2(-0.74, 0.62), vec2(-0.74 + 0.22*s, 0.0)) - 0.09);
    float tw = 0.2*sin(t*3.0);
    d = min(d, sdSeg(q, vec2(-0.9, 0.85), vec2(-1.3, 1.25 + tw)) - 0.07);
    d = min(d, sdSeg(q, vec2(-1.3, 1.25 + tw), vec2(-1.25, 1.7 + tw)) - 0.065);
  } else if (kind < 2.5){                             // leaping: stretched out, paws reaching
    vec2 r = vec2(q.x*0.97 + q.y*0.26, -q.x*0.26 + q.y*0.97);
    float body = sdEll(r - vec2(0.0, 0.7), vec2(1.05, 0.28));
    d = ct_smin(body, ct_head(r, vec2(1.15, 0.95), 0.3), 0.1);
    d = min(d, sdSeg(r, vec2(0.7, 0.6), vec2(1.45, 0.45)) - 0.075);       // front paws forward
    d = min(d, sdSeg(r, vec2(0.6, 0.55), vec2(1.3, 0.3)) - 0.07);
    d = min(d, sdSeg(r, vec2(-0.7, 0.6), vec2(-1.45, 0.35)) - 0.085);     // back legs pushing
    d = min(d, sdSeg(r, vec2(-0.6, 0.55), vec2(-1.3, 0.15)) - 0.08);
    d = min(d, sdSeg(r, vec2(-0.95, 0.8), vec2(-1.7, 1.1)) - 0.065);      // tail streaming
  } else {                                            // curled asleep, tail wrapped round
    float breathe = 1.0 + 0.04*sin(t*1.5);
    float body = sdEll(q - vec2(-0.1, 0.45), vec2(0.9, 0.45*breathe));
    // the head tucked in at the front, ears up, resting on its paws
    float head = ct_head(q, vec2(0.62, 0.55), 0.34);
    d = ct_smin(body, head, 0.08);
    d = min(d, sdEll(q - vec2(0.75, 0.12), vec2(0.35, 0.1)));            // front paws
    // the tail wrapped round the front, its tip curling up
    d = min(d, sdSeg(q, vec2(-0.95, 0.25), vec2(-0.6, 0.02)) - 0.09);
    d = min(d, sdSeg(q, vec2(-0.6, 0.02), vec2(0.35, -0.02)) - 0.085);
    d = min(d, sdSeg(q, vec2(0.35, -0.02), vec2(0.6, 0.12)) - 0.08);
  }
  return d;
}
// cat i: its place (xy), depth (z), heading sign (s), pose kind, size
vec4 ct_place(float fi, out float kind, out float size, out float hs){
  float L = uLength;
  kind = mod(fi, 4.0);
  size = mix(1.6, 2.1, h11(fi*3.3));
  float R = 4.2 + 1.4*h11(fi*5.1);
  float sp = (0.28 + 0.12*h11(fi*2.7))*(kind > 2.5 ? 0.5 : 1.0);
  float a = uTime*sp + fi*1.37;
  float y = L*(0.15 + 0.75*fract(fi*0.618 + 0.21)) + 1.5*sin(uTime*0.4 + fi);
  // swing: they pounce outward
  float pounce = smoothstep(0.0, 1.0, uSwing);
  R += 2.0*pounce;
  // ignition: they spring out from the blade
  R *= smoothstep(0.0, 1.6, uAge - 0.12*fi)*0.85 + 0.15;
  vec2 p = vec2(R*sin(a), y + 0.6*sin(a)*0.0);
  hs = cos(a) >= 0.0 ? 1.0 : -1.0;                   // moving right in front, left behind
  return vec4(p, cos(a), 0.0);
}
vec3 ct_color(float fi){
  float k = mod(fi, 6.0);
  if (k < 1.0) return vec3(1.0, 0.55, 0.8);
  if (k < 2.0) return vec3(0.5, 1.0, 0.8);
  if (k < 3.0) return vec3(1.0, 0.82, 0.4);
  if (k < 4.0) return vec3(0.75, 0.6, 1.0);
  if (k < 5.0) return vec3(0.5, 0.8, 1.0);
  return vec3(1.0, 0.7, 0.55);
}
// a paw print at c (pad + four toes), pointing along dir, size s
float ct_paw(vec2 b, vec2 c, float s){
  vec2 q = (b - c)/s;
  float d = sdEll(q - vec2(0.0, -0.1), vec2(0.42, 0.34));
  d = min(d, length(q - vec2(-0.42, 0.35)) - 0.15);
  d = min(d, length(q - vec2(-0.15, 0.55)) - 0.16);
  d = min(d, length(q - vec2(0.15, 0.55)) - 0.16);
  d = min(d, length(q - vec2(0.42, 0.35)) - 0.15);
  return d*s;
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float L = uLength;
  px = min(px, 0.06);
  float on = smoothstep(-0.6, 0.3, b.y);
  float edge = 1.0 - smoothstep(9.2, 10.6, abs(b.x));
  // the blade: white core, soft lavender glow with a slow moonlight shimmer
  float yc = clamp(b.y, 0.0, L - 0.35);
  float d = length(vec2(b.x, b.y - yc));
  vec3 light = vec3(1.0, 0.97, 1.0)*exp(-pow(d/0.36, 2.0))*1.5*on;
  light += vec3(0.75, 0.55, 1.0)*halo(d, 0.85)*(0.55 + 0.1*sin(b.y*0.5 - uTime*1.5))*on + vec3(0.6, 0.45, 1.0)*halo(d, 2.0)*0.1*on;
  // the cats: neon outlines with a soft fill, dimmer when behind the blade
  for (int i = 0; i < 6; i++){
    float fi = float(i);
    float kind, size, hs;
    vec4 P = ct_place(fi, kind, size, hs);
    vec2 q = (b - P.xy)/size;
    if (abs(q.x) > 2.4 || q.y < -1.2 || q.y > 1.9) continue;
    q.x *= hs;
    q += vec2(0.0, 0.9);
    float cd = ct_cat(q, kind, uTime + fi*1.3)*size;
    float depth = mix(0.45, 1.0, smoothstep(-0.4, 0.4, P.z));
    vec3 cc = ct_color(fi);
    float line = exp(-pow(cd/0.06, 2.0))*1.6 + halo(abs(cd), 0.25)*0.35;
    float fillc = fillD(cd, px)*0.3;
    // eyes: two bright slits, blinking now and then
    vec2 hc = kind < 0.5 ? vec2(0.28, 1.55) : (kind < 1.5 ? vec2(1.02, 1.1) : (kind < 2.5 ? vec2(1.12, 1.25) : vec2(0.62, 0.55)));
    float blink = kind > 2.5 ? 0.0 : step(0.08, fract(uTime*0.3 + fi*0.37));
    float eye = (fillD(length((q - hc - vec2(0.12, 0.05))*vec2(3.0, 1.2)) - 0.12, px*3.0/size) + fillD(length((q - hc - vec2(-0.08, 0.05))*vec2(3.0, 1.2)) - 0.1, px*3.0/size))*blink;
    if (kind > 2.5){                                    // asleep: two closed-eye arcs
      vec2 e1 = q - hc - vec2(0.13, 0.02), e2 = q - hc - vec2(-0.07, 0.02);
      float arc = min(abs(length(e1 + vec2(0.0, 0.08)) - 0.09) + 10.0*step(0.0, -e1.y - 0.0), abs(length(e2 + vec2(0.0, 0.08)) - 0.08) + 10.0*step(0.0, -e2.y));
      eye = halo(arc*size, 0.03)*0.8;
    }
    light += (cc*(line + fillc) + vec3(1.0, 1.0, 0.85)*eye*1.4)*depth;
    // a little Zz over the sleeper
    if (kind > 2.5){
      vec2 zq = q - vec2(0.9, 1.3 + 0.4*fract(uTime*0.5));
      float z = min(min(sdSeg(zq, vec2(0.0, 0.2), vec2(0.2, 0.2)), sdSeg(zq, vec2(0.2, 0.2), vec2(0.0, 0.0))), sdSeg(zq, vec2(0.0, 0.0), vec2(0.2, 0.0)));
      light += cc*halo(z*size, 0.04)*(1.0 - fract(uTime*0.5))*depth;
    }
  }
  // paw prints walking up the blade's edge, each lighting then fading
  float stepL = 1.7;
  float n = floor((b.y + 1.0)/stepL);
  for (int k = 0; k < 2; k++){
    float nk = n + float(k);
    float side = mod(nk, 2.0) < 0.5 ? 1.0 : -1.0;
    vec2 c = vec2(side*1.05, nk*stepL - 0.2);
    float walker = mod(uTime*4.0, L/stepL + 10.0) - 3.0;          // the stroller's position, in steps
    float age = walker - nk;
    float vis = smoothstep(0.0, 0.2, age)*(1.0 - smoothstep(2.0, 6.0, age))*step(0.0, c.y)*step(c.y, L - 1.0);
    float pd = ct_paw(b, c, 0.55);
    light += vec3(1.0, 0.7, 0.95)*(fillD(pd, px)*0.9 + halo(max(pd, 0.0), 0.12)*0.4)*vis;
  }
  // cat eyes blinking open in the dark round the blade
  for (int k = 0; k < 4; k++){
    float fk = float(k);
    float ph = uTime*0.22 + h11(fk*4.1);
    float cy = floor(ph), f = fract(ph);
    vec2 c = vec2((h11(cy*3.3 + fk) < 0.5 ? -1.0 : 1.0)*(4.5 + 2.0*h11(cy*1.7 + fk)), (0.15 + 0.75*h11(cy*2.9 + fk*3.0))*L);
    float open = smoothstep(0.1, 0.2, f)*(1.0 - smoothstep(0.55, 0.65, f));
    open *= 1.0 - step(0.3, f)*step(f, 0.33);                    // one blink
    vec3 ec = h11(cy + fk*7.0) < 0.5 ? vec3(0.7, 1.0, 0.4) : vec3(1.0, 0.8, 0.3);
    for (int e = 0; e < 2; e++){
      vec2 q = b - c - vec2((float(e) - 0.5)*0.75, 0.0);
      float eyeD = length(q*vec2(1.0, 1.0/max(open, 0.05))) - 0.2;
      float slit = abs(q.x) - 0.04;
      light += ec*(fillD(eyeD, px)*(0.3 + 0.7*smoothstep(0.0, 0.05, slit)) + halo(max(eyeD, 0.0), 0.25)*0.4)*open*1.3;
    }
  }
  // clash: nine cat-spirit heads burst out of the strike point and fade
  if (uClash > 0.01){
    float a = 1.0 - uClash;
    vec2 o = vec2(0.0, L*0.6);
    light += vec3(0.9, 0.8, 1.0)*halo(length(b - o), 1.2)*pow(uClash, 3.0)*1.2;
    for (int k = 0; k < 9; k++){
      float fk = float(k);
      float an = fk*0.698 + 0.2;
      vec2 c = o + vec2(cos(an), sin(an))*(0.6 + 6.5*(1.0 - exp(-3.0*a)));
      float hd = ct_head(b, c, 0.55);
      light += ct_color(fk)*(exp(-pow(hd/0.06, 2.0))*1.5 + halo(abs(hd), 0.2)*0.3)*uClash;
    }
  }
  light *= edge*(1.0 - smoothstep(L + 2.0, L + 5.5, b.y))*smoothstep(-3.5, -1.0, b.y);
  return (1.0 - exp(-light*1.15))*1.05;
}
vec3 sig_core(vec3 c, vec2 b, float px){ return vec3(0.0); }
