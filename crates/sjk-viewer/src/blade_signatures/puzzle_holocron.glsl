// Puzzle Holocron: a Jedi holocron's puzzle as a blade. The blade is cut into segments with glowing
// seams; one at a time a segment lights, shifts and unlocks, light seeping out round it and a ring
// of light opening at each of its seams; small motes of holocron light rise off it. A clash flares it.
// @part Unlocking light
// @part Seam rings
// @part Holocron motes
// @room 8
// @palette {"corona.rim_cool":[0.3,0.3,0.4],"corona.rim_hot":[0.85,0.85,0.95],"corona.inner":[0.95,0.95,1.0],"core.white":[1.0,1.0,1.0],"core.fringe_cool":[0.8,0.8,0.95],"core.fringe_hot":[0.95,0.95,1.0],"light.color":[0.9,0.9,1.0],"trail":[0.85,0.85,1.0]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float seg = floor(b.y/3.0);
  float cyc = uTime*0.25;
  float k = floor(cyc), q = fract(cyc);
  float active = 1.0 - step(0.5, abs(seg - mod(k*5.0, 13.0)));
  float turn = sin(3.1416*smoothstep(0.0, 0.6, q))*active;
  float seam = halo((0.5 - abs(fract(b.y/3.0) - 0.5))*3.0, 0.05);
  float slide = halo(abs(b.x - 0.25*turn), 0.06)*active;
  return recol(g, vec3(0.6, 0.8, 1.0))*(0.85 + 0.25*h11(seg)) + lvl(g)*vec3(0.7, 0.9, 1.0)*(seam*1.2 + slide*0.8)*part(0.0);
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 c = vec3(0.6, 0.8, 1.0);
  float k = floor(t*0.25), q = fract(t*0.25);
  float ya = 3.0*mod(k*5.0, 13.0);
  float open = sin(3.1416*q);
  if (ya >= y0 - 3.0){
    fx += fxAura(ro, rd, ya, ya + 3.0, 1.5, c)*open*1.4*part(0.0);
    // the unlocking segment's two seams open into rings of light
    float r = 0.6 + 1.4*smoothstep(0.0, 0.6, q);
    float d = min(ringD(ro, rd, ya, r), ringD(ro, rd, ya + 3.0, r));
    fx += (vec3(1.0)*halo(d, max(0.04, px))*0.9 + c*halo(d, 0.3)*0.5)*open*part(1.0);
  }
  fx += fxMotes(ro, rd, px, y0, y1, 0.6, 2.4, 14.0, 0.035, 4.2, c)*0.8*part(2.0);
  // round 14 (Creyon): a chroma for the blade only; its effects keep their own colours
  uKeep += fx*uFx;
  return vec3(0.0);
}
