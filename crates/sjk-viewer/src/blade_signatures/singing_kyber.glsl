// Singing Kyber (was Resonance): a kyber crystal that sings, as kyber does. a singing crystal blade. The thin teal crystal hums so fast its edges double, a standing
// wave shows bright nodes down its length, and from each node sound rings ripple out and fade. Two
// glowing waveform traces run up beside the blade like an oscilloscope, and tiny crystal shards hover
// round it, trembling to the note. A swing raises the pitch (tighter waves, the traces swell); a clash
// is a sonic boom: a shock cone and a burst of shards. No blinking tip.
// @room 8
// @palette {"corona.rim_cool":[0.11,0.50,0.47],"corona.rim_hot":[0.2,0.9,0.85],"corona.inner":[0.9,1.0,1.0],"core.white":[1.0,1.0,1.0],"core.fringe_cool":[0.12,0.54,0.51],"core.fringe_hot":[0.2,0.9,0.85],"light.color":[0.2,0.9,0.85],"trail":[0.2,0.9,0.85]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float L = uLength;
  px = min(px, 0.06);
  float sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float pitch = 1.0 + 0.9*sw;
  vec3 teal = vec3(0.2, 0.92, 0.86), pale = vec3(0.85, 1.0, 1.0), deep = vec3(0.1, 0.55, 0.75);
  float on = smoothstep(-0.5, 0.3, b.y);
  float yc = clamp(b.y, 0.0, L - 0.35);
  float d = length(vec2(b.x, b.y - yc));
  // ---- the crystal: a white core, edges doubled by the vibration, a standing wave along it
  float nodes = 7.0;
  float sw_ = sin(b.y/L*3.14159*nodes);
  float amp = abs(sw_)*(0.16 + 0.1*sw);                          // the antinodes swell, nodes stay still
  float vib = sin(uTime*90.0);
  float d1 = length(vec2(b.x - amp*vib, b.y - yc)), d2 = length(vec2(b.x + amp*vib, b.y - yc));
  vec3 light = pale*exp(-pow(d/0.3, 2.0))*1.3*on;
  light += teal*(exp(-pow(d1/0.22, 2.0)) + exp(-pow(d2/0.22, 2.0)))*0.55*on;
  light += teal*halo(d, 0.8)*0.5*on + deep*halo(d, 2.0)*0.08*on;
  // the nodes: bright still points on the standing wave
  for (int k = 1; k < 7; k++){
    float y = float(k)/nodes*L;
    light += pale*halo(length(vec2(b.x, (b.y - y)*1.6)), 0.25)*0.9;
  }
  // ---- sound rings rippling out from the nodes, one after another
  for (int j = 0; j < 6; j++){
    float fj = float(j);
    float age = fract(uTime*0.7 + fj/6.0);
    float k = 1.0 + floor(h11(fj*3.7 + floor(uTime*0.7 + fj/6.0))*6.0);
    vec2 o = vec2(0.0, k/nodes*L);
    float rad = 0.6 + 6.0*age;
    float ring = abs(length((b - o)*vec2(1.0, 1.15)) - rad);
    float fade = (1.0 - age)*smoothstep(0.0, 0.06, age);
    float wav = 0.6 + 0.4*sin(atan(b.y - o.y, b.x)*12.0 + uTime*4.0);   // the ring carries its own ripple
    light += teal*(halo(ring, 0.05 + px) + halo(ring, 0.35)*0.2)*fade*wav*1.2;
  }
  // ---- oscilloscope traces running up both sides of the blade
  for (int s = 0; s < 2; s++){
    float sd = s == 0 ? 1.0 : -1.0;
    float x0 = sd*(3.4 + 0.4*sin(uTime*0.6 + sd));
    float env = 0.5 + 0.5*sin(b.y*0.12 - uTime*1.5 + sd);
    float w = (0.45 + 0.35*sw)*env*sin(b.y*(1.4*pitch) - uTime*9.0 + sd*1.3) + 0.05*sin(b.y*4.3*pitch - uTime*17.0);
    float dd = abs(b.x - x0 - w);
    float span = smoothstep(1.0, 4.0, b.y)*(1.0 - smoothstep(L - 4.0, L - 0.5, b.y));
    light += mix(teal, pale, 0.3)*(exp(-pow(dd/0.045, 2.0))*1.4 + halo(dd, 0.25)*0.25)*span;
  }
  // ---- crystal shards hovering round the blade, trembling to the note and rising slowly
  for (int i = 0; i < 9; i++){
    float fi = float(i);
    float ph = uTime*0.12 + h11(fi*5.3);
    float f = fract(ph);
    float sd = mod(fi, 2.0) < 0.5 ? 1.0 : -1.0;
    vec2 p = vec2(sd*(1.6 + 1.3*h11(fi*2.1)) + 0.06*sin(uTime*70.0 + fi), (0.08 + 0.8*fract(h11(fi*7.7) + f*0.5))*L);
    p.x += sd*sw*3.0;
    vec2 q = b - p;
    float an = uTime*(1.0 + h11(fi)) + fi;
    q = vec2(q.x*cos(an) - q.y*sin(an), q.x*sin(an) + q.y*cos(an));
    float sh = abs(q.x)/0.16 + abs(q.y)/0.42 - 1.0;
    float glint = pow(0.5 + 0.5*sin(uTime*3.0 + fi*2.0), 8.0);
    light += mix(teal, pale, glint)*(clamp(-sh*2.0, 0.0, 1.0)*(0.8 + 1.2*glint) + halo(max(sh, 0.0)*0.2, 0.08)*0.5);
  }
  // ---- clash: a sonic boom, a cone of shock lines and a burst of shards
  if (cl > 0.01){
    float a = 1.0 - cl;
    vec2 o = vec2(0.0, L*0.6);
    for (int k = 0; k < 3; k++){
      float rad = 1.0 + 7.0*a - float(k)*0.9;
      float ring = abs(length(b - o) - rad);
      light += pale*halo(ring, 0.08)*cl*(1.0 - float(k)*0.3)*step(0.0, rad);
    }
    for (int i = 0; i < 10; i++){
      float fi = float(i);
      float an = fi*0.628 + 0.3*h11(fi);
      vec2 p = o + vec2(cos(an), sin(an))*(0.5 + 6.5*(1.0 - exp(-3.0*a)));
      vec2 q = b - p;
      float sh = abs(q.x)/0.14 + abs(q.y)/0.4 - 1.0;
      light += pale*clamp(-sh*2.0, 0.0, 1.0)*cl*1.4;
    }
    light += teal*halo(length(b - o), 1.6)*pow(cl, 3.0);
  }
  float edge = 1.0 - smoothstep(8.5, 10.2, abs(b.x));
  light *= edge*(1.0 - smoothstep(L + 0.5, L + 3.5, b.y))*smoothstep(-3.0, -0.5, b.y);
  return (1.0 - exp(-light*1.15))*1.05;
}
vec3 sig_core(vec3 c, vec2 b, float px){ return vec3(0.0); }
