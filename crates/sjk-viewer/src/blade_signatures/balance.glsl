// Balance (was Paradox): the light and the dark side of the Force in one blade: a crimson strand
// and a pale blue-white strand lying side by side inside it. Twists run up the blade and the two swap places, passing through each other where
// they meet, and there they cancel into a black knot rimmed with white. Every nine seconds the whole
// blade turns inside out for a moment: its core goes black and its colours trade sides. Clashes swap
// them at once; swinging twists them faster.
// @room 8
// @solid 1
// @palette {"corona.rim_cool":[0.45,0.70,1.0],"corona.rim_hot":[1.0,0.12,0.08],"corona.inner":[1.0,0.92,0.85],"core.white":[1.0,1.0,1.0],"core.fringe_cool":[0.45,0.75,1.0],"core.fringe_hot":[1.0,0.10,0.08],"light.color":[0.85,0.55,0.75],"trail":[0.9,0.45,0.6]}

const vec3 PD_A = vec3(1.0, 0.08, 0.06);
const vec3 PD_B = vec3(0.45, 0.75, 1.0);
float pd_ph(){ return mod(uTime - 10.8, 9.0); }
float pd_inv(){ float p = pd_ph(); return smoothstep(0.0, 0.15, p)*(1.0 - smoothstep(1.2, 1.5, p)); }
// the twist angle at height y: a staircase in time, so the strands swap and rest in turn
float pd_phi(float y){
  float u = uTime*(0.28) - y/15.0 + uSwing*0.6;
  float s = floor(u) + smoothstep(0.30, 0.70, fract(u));
  return 3.14159*(s + uClash);
}
float pd_on(vec2 b){ return smoothstep(-0.1, 0.4, b.y)*(1.0 - smoothstep(uLength - 0.6, uLength + 0.3, b.y)); }
// rgb light of both strands; a = how close the crossing is (1 at the knot)
vec4 pd_strands(vec2 b, float px){
  float phi = pd_phi(b.y);
  float W = 0.40;
  float xa = W*cos(phi), za = sin(phi);
  float inv = pd_inv();
  vec3 ca = mix(PD_A, PD_B, inv), cb = mix(PD_B, PD_A, inv);
  float da = abs(b.x - xa), db = abs(b.x + xa);
  float wa = 0.13 + 0.03*za, wb = 0.13 - 0.03*za;
  vec3 A = ca*(halo(da, 0.33)*0.9 + fillD(da - wa, px)*0.9) + mix(ca, vec3(1.0), 0.6)*fillD(da - wa*0.45, px)*0.9;
  vec3 B = cb*(halo(db, 0.33)*0.9 + fillD(db - wb, px)*0.9) + mix(cb, vec3(1.0), 0.6)*fillD(db - wb*0.45, px)*0.9;
  // the one behind is dimmer
  A *= mix(0.55, 1.0, smoothstep(-0.6, 0.6, za)); B *= mix(0.55, 1.0, smoothstep(-0.6, 0.6, -za));
  float cross = 1.0 - smoothstep(0.05, 0.32, abs(xa));
  return vec4((A + B)*pd_on(b), cross);
}
vec3 paradox_glow0(vec3 g, vec2 b, float r, float px){
  float L = uLength;
  vec4 s = pd_strands(b, px);
  float inv = pd_inv();
  // each side of the glow takes its strand's colour
  float phi = pd_phi(b.y);
  float side = cos(phi)*sign(b.x + 1e-4);
  vec3 left = mix(PD_A, PD_B, inv), right = mix(PD_B, PD_A, inv);
  vec3 tint = mix(right, left, smoothstep(-0.4, 0.4, side));
  float bd = length(vec2(b.x, max(b.y - L, 0.0) + max(-b.y, 0.0)*4.0));
  vec3 glow = tint*(halo(bd, 1.1)*0.45 + halo(bd, 2.3)*0.2)*smoothstep(-0.3, 0.3, b.y);
  // the crossing: the two cancel, a rim of white light round the void
  float kn = s.a;
  vec3 rim = vec3(1.0)*halo(abs(abs(b.x) - 0.42), 0.06 + px)*kn*1.4*pd_on(b);
  // a sparkle of paradox motes leaking from each knot
  float mote = 0.0;
  for (int i = 0; i < 6; i++){
    float fi = float(i);
    float age = fract(uTime*0.7 + h11(fi*3.3));
    float y = h11(fi + floor(uTime*0.7 + h11(fi*3.3))*7.1)*L;
    float side2 = (h11(fi*5.1) < 0.5 ? -1.0 : 1.0);
    vec2 p = vec2(side2*(0.6 + age*1.4), y + age*1.2);
    mote += fillD(length(b - p) - 0.05*(1.0 - age), px)*(1.0 - age)*(1.0 - smoothstep(0.05, 0.3, abs(cos(pd_phi(y)))));
  }
  return glow*(1.0 - 0.6*kn*pd_on(b)) + s.rgb*(1.0 - 0.8*kn) + rim + vec3(0.9, 0.95, 1.0)*mote*1.2
       + mix(vec3(1.0), tint, 0.4)*uClash*halo(bd, 1.3)*0.8 + g*0.05;
}
vec3 sig_core(vec3 c, vec2 b, float px){ return vec3(0.0); }
vec4 sig_solid(vec4 o, vec2 b, float px){
  vec4 s = pd_strands(b, px);
  float on = pd_on(b);
  // the black knot where the strands pass through each other
  float kn = smoothstep(0.35, 0.9, s.a)*(1.0 - smoothstep(0.30, 0.42, abs(b.x)))*on;
  // inside out: the blade's own core goes black for a moment
  float inv = pd_inv();
  float neg = inv*(1.0 - smoothstep(0.10, 0.22, abs(b.x)))*on;
  float a = max(kn*0.92, neg*0.95);
  return vec4(mix(o.rgb, vec3(0.0), a), max(o.a, a));
}

// ==== show (added) ====
// Paradox show: fire and ice off the two strands: on one side molten flames and embers lick away,
// on the other frost crystals and cold mist drift off; every few seconds they swap sides, passing
// through the blade. Impossible triangles float round it, turning. A swing throws fire and frost
// out together; a clash cancels them in a black-and-white flash ring.
vec3 paradox_show(vec2 b, float px){
  float L = uLength, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  vec3 fire = vec3(1.0, 0.12, 0.08), ice = vec3(0.6, 0.85, 1.0);
  vec3 light = vec3(0.0);
  float swap = sign(sin(uTime*0.7));
  float sm = smoothstep(-0.3, 0.3, sin(uTime*0.7));
  for (int i = 0; i < 12; i++){
    float fi = float(i);
    float ph = uTime*(0.5 + 0.25*h11(fi*2.3)) + h11(fi*6.7);
    float cy = floor(ph), f = fract(ph);
    float isFire = mod(fi, 2.0);
    float s = (isFire > 0.5 ? 1.0 : -1.0)*mix(-1.0, 1.0, sm);
    float y0 = (0.08 + 0.85*h11(cy*1.7 + fi))*L;
    float fade = (1.0 - f)*smoothstep(0.0, 0.08, f);
    if (isFire > 0.5){
      vec2 p = vec2(s*(0.5 + 4.0*f) + 0.4*sin(f*9.0 + fi), y0 + 3.5*f);
      light += mix(vec3(1.0, 0.9, 0.5), fire, f)*(halo(length(b - p), 0.12)*2.2 + halo(length(b - p), 0.45)*0.35)*fade;
      float u = (b.x*s - 0.4)/2.4;
      if (u > 0.0 && u < 1.0){
        float dy = b.y - y0 - 2.0*u*u - 0.4*sin(u*8.0 - uTime*11.0)*u;
        light += fire*exp(-pow(dy/(0.28*(1.0 - u) + 0.04), 2.0))*(1.0 - u)*fade*1.2;
      }
    } else {
      vec2 p = vec2(s*(0.6 + 4.5*f), y0 - 1.5*f);
      vec2 q = (b - p)/1.3;
      float r = length(q), a = atan(q.y, q.x) + uTime + fi;
      float arm = halo(abs(sin(a*3.0))*r, 0.03)*step(r, 0.4);
      light += ice*(arm*1.5 + halo(r, 0.07)*1.2)*fade + ice*halo(length(b - p), 1.0)*0.08*fade;
    }
  }
  for (int i = 0; i < 3; i++){
    float fi = float(i);
    vec2 c = vec2((mod(fi, 2.0) < 0.5 ? -1.0 : 1.0)*(4.0 + 0.5*sin(uTime + fi)), (0.25 + 0.25*fi)*L);
    vec2 q = b - c;
    float an = uTime*0.6 + fi*2.0;
    q = vec2(q.x*cos(an) - q.y*sin(an), q.x*sin(an) + q.y*cos(an));
    float d = 1e3;
    for (int k = 0; k < 3; k++){
      float a0 = float(k)*2.094, a1 = a0 + 2.094;
      vec2 p0 = vec2(cos(a0), sin(a0))*1.1, p1 = vec2(cos(a1), sin(a1))*1.1;
      vec2 p2 = p0*0.55, p3 = p1*0.55 + (p0 - p1)*0.12;
      d = min(d, min(sdSeg(q, p0, p1), sdSeg(q, p2, p3)));
      d = min(d, sdSeg(q, p0, p2 + (p1 - p0)*0.1));
    }
    light += mix(fire, ice, 0.5 + 0.5*sin(uTime + fi))*(exp(-pow(d/0.04, 2.0))*1.2 + halo(d, 0.2)*0.15);
  }
  if (cl > 0.01){
    vec2 o = vec2(0.0, L*0.6);
    float rr = abs(length(b - o) - 1.0 - 7.0*(1.0 - cl));
    light += vec3(1.0)*halo(rr, 0.1)*cl*1.6 + mix(fire, ice, step(0.0, b.x - o.x))*halo(rr - 0.4, 0.2)*cl;
  }
  float edge = 1.0 - smoothstep(8.5, 10.2, abs(b.x));
  light *= edge*(1.0 - smoothstep(L + 0.5, L + 3.5, b.y))*smoothstep(-3.0, -0.5, b.y);
  return (1.0 - exp(-light*1.15))*1.05;
}

// ==== the two sides (round 7) ====
// The dark side throws crimson lightning off its side of the blade, forks leaping 5 to 8 units out
// and struck anew many times a second; the light side breathes calm blue-white wisps of the Force
// that rise and curl away on the other. Two orbs, one of each, circle the blade in step like a
// balance; every swap a ring of both colours ripples out.
vec3 pd_sides(vec2 b, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0);
  vec3 red = vec3(1.0, 0.1, 0.06), pale = vec3(0.65, 0.85, 1.0);
  float sm = smoothstep(-0.3, 0.3, sin(t*0.7));
  float darkSide = mix(-1.0, 1.0, sm);                 // the side the dark strand throws to
  vec3 light = vec3(0.0);
  for (int i = 0; i < 5; i++){
    float fi = float(i);
    float k = floor(t*8.0 + fi*0.31);
    float y0 = (0.1 + 0.8*h11(k*1.3 + fi))*L;
    vec2 end = vec2(darkSide*(5.0 + 3.0*h11(k*2.1 + fi)), y0 + (h11(k*3.3 + fi) - 0.4)*5.0);
    float on = step(0.4 - 0.35*sw, h11(k*4.7 + fi));
    float d = boltD(b, vec2(darkSide*0.3, y0), end, k + fi, 0.32);
    // a fork off the bolt halfway
    vec2 mid = mix(vec2(darkSide*0.3, y0), end, 0.5) + vec2(0.0, (h11(k*5.1 + fi) - 0.5)*1.5);
    d = min(d, boltD(b, mid, mid + vec2(darkSide*2.5, 2.0*(h11(k*6.3 + fi) - 0.5)), k + fi + 0.3, 0.35));
    light += boltLight(d, red, px)*on;
  }
  for (int i = 0; i < 9; i++){
    float fi = float(i);
    float f = fract(t*0.18 + h11(fi*3.9));
    float y0 = (0.05 + 0.7*h11(fi*2.2))*L;
    vec2 p = vec2(-darkSide*(0.8 + 4.5*f + 0.8*sin(f*6.0 + fi)), y0 + 7.0*f);
    vec2 q = b - p;
    float wisp = halo(length(q*vec2(1.0, 0.5)), 0.35)*(1.0 - f)*smoothstep(0.0, 0.1, f);
    float trail = halo(abs(q.x + 0.8*sin(q.y*1.2 + fi)), 0.12)*step(-3.0, q.y)*step(q.y, 0.0)*(1.0 + q.y/3.0)*(1.0 - f);
    light += pale*(wisp*2.0 + trail*0.8);
  }
  for (int k = 0; k < 2; k++){
    float fk = float(k);
    float a = t*1.1 + fk*3.1416;
    vec2 p = vec2(3.2*sin(a), L*0.5 + 7.0*sin(a*0.5 + fk));
    float front = 0.55 + 0.45*cos(a);
    vec3 c = k == 0 ? red : pale;
    light += (c*halo(length(b - p), 0.55)*1.6 + vec3(1.0)*halo(length(b - p), 0.18)*1.6)*front;
  }
  float ring = abs(length((b - vec2(0.0, L*0.5))*vec2(1.0, 0.45)) - 1.0 - 8.0*fract(t*0.7/3.1416 + 0.25));
  light += mix(red, pale, 0.5 + 0.5*sin(t*0.7))*halo(ring, 0.12)*0.6*(1.0 - fract(t*0.7/3.1416 + 0.25));
  float edge = 1.0 - smoothstep(8.5, 10.0, abs(b.x));
  light *= edge*(1.0 - smoothstep(L + 0.8, L + 3.5, b.y))*smoothstep(-2.5, -0.5, b.y);
  return (1.0 - exp(-light*1.15))*1.05;
}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  px = min(px, 0.06);
  return paradox_glow0(g, b, r, px) + paradox_show(b, px) + pd_sides(b, px);
}
