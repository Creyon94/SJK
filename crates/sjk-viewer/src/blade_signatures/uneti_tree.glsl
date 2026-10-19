// Uneti Tree: the Force-sensitive tree of Ahch-To, where the first Jedi texts were kept, as a blade
// of living wood-light. The blade has no plain tube: its body is grown from luminous fibres that
// braid and twist round a bright heartwood core, like the grain of an ancient trunk made of light.
// Sap of the Force flows up through the fibres in slow bright pulses, the bark-like skin breathes
// and glows through its grain, and soft motes of Force pollen drift up round it in slow spirals
// inside a gentle living aura. Swinging quickens the sap; a clash floods every fibre with light.
// @room 7
// @palette {"corona.rim_cool":[0.06,0.20,0.55],"corona.rim_hot":[0.35,0.70,1.0],"corona.inner":[0.75,0.90,1.0],"core.white":[0.92,0.97,1.0],"core.fringe_cool":[0.30,0.60,1.0],"core.fringe_hot":[0.70,0.88,1.0],"light.color":[0.40,0.70,1.0],"trail":[0.40,0.70,1.0]}

const vec3 UT_COL = vec3(0.22, 0.55, 1.0);
const vec3 UT_PALE = vec3(0.70, 0.88, 1.0);

float ut_top(){ float g = clamp(uAge/0.45, 0.0, 1.0); return uLength*(1.0 - (1.0 - g)*(1.0 - g)); }
// sap pulses climbing the blade at height y (0..1, sharp leading edge, long soft tail)
float ut_sap(float y){
  float u = y*0.11 - uTime*(0.55 + 0.9*uSwing);
  float f = fract(u);
  return 1.6*pow(f, 8.0)*smoothstep(1.0, 0.96, f) + 0.6*pow(fract(u*1.7 + 0.37), 12.0);
}
// distance (in units) from a cross-section point to a set of n strands wound at radius rr,
// angle offset ph (turns); a is the point's angle in turns, r its radius
float ut_set(float a, float r, float n, float ph, float rr){
  float u = fract(a*n - ph*n) - 0.5;
  float arc = u/n*6.2832*rr;
  return length(vec2(arc, r - rr));
}
// the living wood: fibre density and its sap brightness at point p
vec2 ut_wood(vec3 p, float top){
  float y = p.y;
  float r = length(p.xz);
  float a = atan(p.x, p.z)/6.2832;
  float tw = y/9.0;
  float br = 0.5 + 0.5*sin(uTime*1.3 + y*0.25);
  // two counter-twisting sets of fibres: a braid; their radius swells and narrows like grain
  float r1 = 0.42 + 0.14*sin(y*0.55 + uTime*0.7);
  float r2 = 0.42 + 0.14*sin(y*0.55 + 3.1416 + uTime*0.7);
  float d1 = ut_set(a, r, 3.0, tw + uTime*0.05, r1);
  float d2 = ut_set(a, r, 3.0, -tw - uTime*0.05 + 0.1667, r2);
  // fine inner fibres twisting tight round the heartwood
  float d3 = ut_set(a, r, 5.0, tw*2.2 - uTime*0.12, 0.2);
  float w = 0.085;
  // loose outer fibres that lift off the body and wind back in, slowly
  float r4 = 0.78 + 0.22*sin(y*0.3 - uTime*0.9 + a*6.2832);
  float d4 = ut_set(a, r, 2.0, tw*0.6 + uTime*0.03 + 0.25, r4);
  float f = exp(-d1*d1/(w*w)) + exp(-d2*d2/(w*w)) + 0.6*exp(-d3*d3/(0.06*0.06)) + 0.45*exp(-d4*d4/(0.06*0.06));
  // the bark: a breathing skin at the body's edge, glowing through streaks of grain
  float grain = fbm3(vec2((a + tw*0.35)*14.0, y*0.35 - uTime*0.25));
  float skin = exp(-pow((r - 0.66 - 0.04*br)/0.09, 2.0))*smoothstep(0.42, 0.78, grain)*(0.5 + 0.7*br);
  float on = smoothstep(-0.7, -0.2, y)*(1.0 - smoothstep(top - 0.9, top + 0.3, y));
  float sap = ut_sap(y + a*2.0);
  return vec2((f*(0.9 + 1.8*sap) + skin*0.9)*on, (f*sap)*on);
}
// one mote of Force pollen drifting up in a spiral
vec3 ut_mote(vec3 ro, vec3 rd, float i, float px, float top){
  float sp = 0.045 + 0.04*h11(i*3.7);
  float ph = fract(h11(i*1.31) + uTime*sp*(1.0 + uSwing));
  float y = ph*(uLength + 3.0) - 1.0;
  if (y > top) return vec3(0.0);
  float rad = 1.0 + 3.0*h11(i*7.9) + 0.8*ph;
  float ang = h11(i*5.3) + ph*(1.2 + h11(i*2.2)) + uTime*0.03;
  vec3 c = around(rad, ang, y);
  float t; float d = rayPt(ro, rd, c, t);
  float life = smoothstep(0.0, 0.12, ph)*(1.0 - smoothstep(0.75, 1.0, ph));
  float tw = 0.6 + 0.4*sin(uTime*(3.0 + 4.0*h11(i)) + i);
  float s = max(px*0.9, 0.07);
  return (vec3(1.0)*halo(d, s)*1.1 + UT_COL*halo(d, 0.3)*0.55 + UT_COL*halo(d, 0.9)*0.08)*life*tw;
}
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float top = ut_top();
  float cl = clamp(uClash, 0.0, 1.0);
  vec3 col = vec3(0.0);
  // heartwood core: a slim white-blue line inside the fibres, filling the emitter
  float sa;
  float dax = raySeg(ro, rd, vec3(0.0, -0.6, 0.0), vec3(0.0, top, 0.0), sa);
  col += vec3(1.0)*exp(-pow(dax/max(0.16, px*0.7), 2.0))*1.1;
  col += mix(UT_COL, UT_PALE, 0.4)*halo(dax, 0.5)*0.55;
  // the living aura: breathes slowly
  float breathe = 0.75 + 0.25*sin(uTime*1.3 + ro.y*0.0);
  col += UT_COL*halo(dax, 2.2)*0.22*breathe*(1.0 + uSwing*0.6 + cl*1.5);
  col += UT_COL*exp(-dax/4.0)*0.05*uFx;
  // the fibre body, marched
  if (dax < 1.3){
    vec3 ms = marchSteps(ro, rd, 1.1, -0.6, top + 0.4, 22.0);
    if (ms.z > 0.0){
      float dt = ms.z;
      float t = ms.x + dt*h21(rd.xy*517.0 + rd.z*91.0);
      vec3 acc = vec3(0.0);
      for (int i = 0; i < 22; i++){
        vec3 p = ro + rd*t;
        vec2 w = ut_wood(p, top);
        acc += (UT_COL*w.x*1.2 + UT_PALE*w.y*1.6 + vec3(1.0)*w.y*w.y*0.6)*dt;
        t += dt;
      }
      // keep long end-on paths from blowing out
      col += acc*(3.6/(1.0 + length(acc)*0.25))*(1.0 + cl*1.5);
    }
  }
  // the growing tip: a bright bud of light while it ignites
  float tt; float dt0 = rayPt(ro, rd, vec3(0.0, top, 0.0), tt);
  float growing = 1.0 - smoothstep(0.4, 0.7, uAge);
  col += (vec3(1.0)*halo(dt0, 0.35)*1.2 + UT_COL*halo(dt0, 1.4)*0.6)*growing;
  // Force pollen
  vec3 m = vec3(0.0);
  if (dax < 6.5){
    for (int i = 0; i < 28; i++){ m += ut_mote(ro, rd, float(i), px, top); }
  }
  col += m*uFx;
  // clash: a ring of light blooms out across the blade
  if (cl > 0.0){
    vec3 h = rayAt(ro, rd, uLength*0.62);
    float rr = length(h.xy);
    float R = 0.6 + (1.0 - cl)*4.5;
    if (h.z > 0.0) col += (UT_PALE*halo(abs(rr - R), 0.35) + vec3(1.0)*halo(abs(rr - R), 0.08))*cl*1.4*uFx;
    col += UT_PALE*halo(dax, 1.5)*cl*0.8;
  }
  return (1.0 - exp(-col*1.1))*1.05;
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){ return vec3(0.0); }
vec3 sig_core(vec3 c, vec2 b, float px){ return vec3(0.0); }
