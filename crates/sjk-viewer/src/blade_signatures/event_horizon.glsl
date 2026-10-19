// Event Horizon: the blade is a black hole stretched into a line. Its heart is a rod of pure
// darkness outlined by a razor-thin photon ring, with light running down its edges as it is
// pulled in. Two real accretion disks of hot swirling matter circle the blade at two heights,
// turning, brighter on the side rushing toward you, each wrapped in a thin lensed photon ring;
// streams of matter spiral in along the blade from the disks in pulses of light. Seen from above
// the disks are full whirlpools inside a bright Einstein ring; from the side, thin blazing lines.
// A swing spins the disks up and speeds the infalling streams; a clash fires a gravity wave.
// @room 9
// @solid 1
// @palette {"corona.rim_cool":[0.30,0.06,0.70],"corona.rim_hot":[0.70,0.40,1.0],"corona.inner":[0.9,0.85,1.0],"core.white":[0.95,0.9,1.0],"core.fringe_cool":[0.40,0.12,0.90],"core.fringe_hot":[0.80,0.55,1.0],"light.color":[0.55,0.25,1.0],"trail":[0.60,0.35,1.0]}

float eh_top(){ return uLength*clamp(uAge/0.45, 0.0, 1.0); }
float eh_h(float k){ return uLength*(0.36 + 0.34*k) + 0.9*sin(uTime*(0.4 - 0.07*k) + k*2.0); }
// how far a disk has grown (it spreads out from the blade as the ignition passes its height)
float eh_grow(float h){ return smoothstep(h - 1.0, h + 5.0, eh_top()); }

// one accretion disk at height h: a thin slab of hot matter marched along the ray, up to tmax
vec3 eh_disk(vec3 ro, vec3 rd, float h, float k, float tmax, float g){
  vec3 vio = vec3(0.62, 0.36, 1.0), white = vec3(1.0, 0.96, 1.0);
  float Ro = 6.2*g;
  vec3 m = marchSteps(ro, rd, Ro, h - 0.7, h + 0.7, 16.0);
  vec3 acc = vec3(0.0);
  if (m.y <= m.x || g < 0.01) return acc;
  float sw = clamp(uSwing, 0.0, 1.0);
  float spin = uTime*(1.7 + 0.5*k + 1.5*sw);
  float t = m.x + m.z*h21(rd.xy*517.0 + k);
  for (int i = 0; i < 16; i++){
    if (t > tmax) break;
    vec3 p = ro + rd*t;
    float r = length(p.xz)/max(g, 0.05);
    float dy = p.y - h;
    float thick = 0.07 + 0.045*r;
    float vert = exp(-dy*dy/(thick*thick));
    if (vert > 0.01 && r > 0.45){
      float a = atan(p.z, p.x);
      // rigid spiral arms and swirling clumps (no runaway shear)
      float arms = pow(0.5 + 0.5*sin(2.0*(a - spin) + 4.5*log(r) + k*1.7), 2.0);
      vec2 q = rot(spin*0.8 + 1.6*log(r))*p.xz;
      float clump = fbm3(vec2(q.x*0.9 + k*7.0, q.y*2.6));
      float lanes = 0.5 + 0.5*sin(r*7.0 - clump*5.0);
      float rad = smoothstep(0.55, 0.85, r)*(1.0 - smoothstep(2.8, 6.0, r));
      float isco = exp(-pow((r - 0.85)/0.12, 2.0));
      float T = clamp(1.25/r, 0.0, 1.0);
      // Doppler: the side whose matter rushes toward the camera is brighter and whiter
      vec3 vt = normalize(vec3(-p.z, 0.0, p.x) + 1e-4);
      float dop = dot(vt, -rd);
      float beam = pow(1.0 + 0.6*dop, 3.0);
      float dens = rad*vert*(0.15 + 0.85*arms)*(0.3 + 1.2*smoothstep(0.35, 0.75, clump))*(0.55 + 0.45*lanes)*(1.0 + 0.6*sw + 1.2*clamp(uClash, 0.0, 1.0));
      dens += isco*vert*1.5;
      vec3 c = mix(vio, white, T*T*(0.6 + 0.4*clamp(dop, 0.0, 1.0)));
      acc += c*dens*beam*(0.6 + 2.6*T*T)*m.z*4.5;
    }
    t += m.z;
  }
  return acc;
}

// all the effects round the blade, counted only up to tmax along the ray
vec3 eh_fx(vec3 ro, vec3 rd, float px, float tmax){
  float L = uLength, top = eh_top();
  float sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  vec3 vio = vec3(0.62, 0.36, 1.0), white = vec3(1.0, 0.96, 1.0);
  vec3 light = vec3(0.0);
  float s, tq;
  for (int k = 0; k < 2; k++){
    float fk = float(k);
    float h = eh_h(fk), g = eh_grow(h);
    if (g < 0.01) continue;
    light += eh_disk(ro, rd, h, fk, tmax, g);
    // the lensed photon ring: a thin ring of bent light hugging the hole at the disk's height,
    // always facing you; brighter when seen from above
    float dc = rayPt(ro, rd, vec3(0.0, h, 0.0), tq);
    if (tq < tmax){
      vec3 q = ro + rd*tq - vec3(0.0, h, 0.0);
      float side = dot(normalize(vec3(-q.z, 0.0, q.x) + 1e-4), -rd);
      float flick = 0.75 + 0.25*sin(atan(q.y, length(q.xz)*sign(q.x + 1e-4))*5.0 - uTime*6.0);
      float rr = abs(dc - 0.95*g);
      float up = 0.55 + 0.9*rd.y*rd.y;
      light += (white*halo(rr, max(0.035, px))*1.8 + vio*halo(rr, 0.25)*0.35)*(0.7 + 0.5*side)*flick*up*g;
      // soft bloom round the whole disk
      light += vio*halo(dc, 3.2*g)*0.07;
    }
    // matter streams spiralling in along the blade from the disk's inner edge, up and down
    float da = raySeg(ro, rd, vec3(0.0, h - 7.5, 0.0), vec3(0.0, h + 7.5, 0.0), s);
    if (da > 2.6) continue;
    for (int j = 0; j < 4; j++){
      float fj = float(j);
      float dir = j < 2 ? 1.0 : -1.0;
      float a0 = fj*0.5 + fk*0.3 - uTime*(0.35 + 0.4*sw);
      vec3 prev = around(1.7*g, a0, h);
      for (int i = 1; i <= 8; i++){
        float u = float(i)/8.0;
        vec3 p = around(mix(1.7, 0.36, sqrt(u))*g, a0 + u*1.6*dir, h + dir*u*7.0*g);
        float d = raySeg(ro, rd, prev, p, s);
        if (d < 0.6){
          float uu = u - (1.0 - s)/8.0;
          vec3 qp = mix(prev, p, s);
          float tt = dot(qp - ro, rd);
          float pulse = 0.35 + 0.65*pow(0.5 + 0.5*sin(uu*14.0 - uTime*(7.0 + 5.0*sw) + fj*1.3), 3.0);
          float fade = (1.0 - smoothstep(0.6, 1.0, uu))*smoothstep(0.0, 0.08, uu);
          vec3 c = mix(vio, white, uu*0.7 + 0.2);
          light += c*(halo(d, max(0.035, px))*1.3 + halo(d, 0.18)*0.35)*pulse*fade*step(tt, tmax);
        }
        prev = p;
      }
    }
  }
  // the Einstein ring: from above (or below) a bright ring of lensed light circles the blade
  float aligned = smoothstep(0.8, 0.97, -rd.y);
  if (aligned > 0.0){
    float hE = eh_h(1.0);
    float dA = rayPt(ro, rd, vec3(0.0, hE, 0.0), tq);
    float er = abs(dA - 3.1 - 0.15*sin(uTime*1.3));
    vec3 pa = ro + rd*tq;
    float ang = atan(pa.z, pa.x);
    float sh = 0.6 + 0.4*sin(ang*3.0 + uTime*2.0);
    light += (white*halo(er, max(0.05, px))*2.2 + vio*halo(er, 0.5)*0.45)*sh*aligned*eh_grow(hE);
  }
  // a clash: a gravity wave (a sphere of bent light) bursts out from the struck point
  if (cl > 0.01){
    float dc = rayPt(ro, rd, vec3(0.0, L*0.6, 0.0), tq);
    float rr = abs(dc - 0.6 - 5.5*(1.0 - cl));
    light += (white*halo(rr, 0.1)*2.2 + vio*halo(rr, 0.8)*0.5)*cl*step(tq, tmax);
  }
  return light;
}

// the blade: the emitter fills with violet-white light, then the black rod with its photon rim,
// light streaming down its edges as it falls in
vec3 eh_blade(vec3 ro, vec3 rd, float px){
  float top = eh_top(), s;
  vec3 vio = vec3(0.62, 0.36, 1.0), white = vec3(1.0, 0.96, 1.0);
  float d = raySeg(ro, rd, vec3(0.0, -0.6, 0.0), vec3(0.0, max(top - 0.3, -0.5), 0.0), s);
  float y = mix(-0.6, top - 0.3, s);
  float stream = 0.65 + 0.35*sin(y*0.9 + uTime*12.0) + 0.3*pow(0.5 + 0.5*sin(y*0.23 + uTime*5.0), 6.0);
  float rim = exp(-pow((d - 0.34)/max(0.06, px), 2.0));
  vec3 light = white*rim*1.6*stream + vio*halo(d, 0.85)*0.6 + vio*halo(d, 2.6)*0.12;
  // the emitter: plain light from inside the hilt up to where the darkness begins
  float sm;
  float de = raySeg(ro, rd, vec3(0.0, -0.6, 0.0), vec3(0.0, min(top, 1.2), 0.0), sm);
  light += white*exp(-pow(de/0.38, 2.0))*1.3*(1.0 - smoothstep(0.6, 1.0, sm*min(top, 1.2)/1.2));
  light += vio*halo(de, 0.9)*0.4;
  float cl = clamp(uClash, 0.0, 1.0);
  light += white*rim*cl*1.5;
  return light;
}

vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 light = eh_blade(ro, rd, px) + eh_fx(ro, rd, px, 1e9)*uFx;
  return (1.0 - exp(-light*1.1))*1.05;
}
// the black rod itself, with any light in front of it drawn over it
vec4 sig_vol_solid(vec3 ro, vec3 rd, float px){
  float top = eh_top(), s;
  if (top < 1.3) return vec4(0.0);
  float d = raySeg(ro, rd, vec3(0.0, 1.0, 0.0), vec3(0.0, top - 0.3, 0.0), s);
  float a = (1.0 - smoothstep(0.24, 0.31, d))*smoothstep(0.0, 0.12, s*(top - 1.3));
  if (a < 0.002) return vec4(0.0);
  vec3 c = vec3(0.0, 1.0 + s*(top - 1.3), 0.0);
  float tc = dot(c - ro, rd);
  vec3 front = eh_fx(ro, rd, px, tc)*uFx;
  vec3 col = vec3(0.01, 0.0, 0.02) + (1.0 - exp(-front*1.1))*1.05;
  return vec4(col, a*0.96);
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){ return vec3(0.0); }
vec3 sig_core(vec3 c, vec2 b, float px){ return vec3(0.0); }
