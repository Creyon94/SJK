// Ashla Radiance: the Ashla, the light side of the Force in the old Jedi teachings, held as a
// blade. A white-gold blade glowing calm and steady, slow waves of light rising through it; soft
// radiant light rises off it like a quiet flame, ribbons of gold light curl slowly up beside it,
// motes of light drift upward round it and a faint echo of the blade sways beside it. Nothing is
// drawn on its tip. A swing brightens the light; a clash flares the blade.
// @part Radiant light
// @part Light ribbons
// @motes Light of the Ashla
// @room 10
// @palette {"corona.rim_cool":[0.55,0.42,0.15],"corona.rim_hot":[1.0,0.86,0.5],"corona.inner":[1.0,0.96,0.82],"core.white":[1.0,0.99,0.94],"core.fringe_cool":[1.0,0.82,0.45],"core.fringe_hot":[1.0,0.95,0.78],"light.color":[1.0,0.86,0.5],"trail":[1.0,0.86,0.55]}

const vec3 AR_GOLD = vec3(1.0, 0.84, 0.5);
const vec3 AR_PALE = vec3(1.0, 0.96, 0.85);

// the round-8 effects: the radiant light rising off the blade like a quiet flame
vec3 r8_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, y0 = L*0.2, y1 = L*0.97;
  float sw = clamp(uSwing, 0.0, 1.0);
  vec3 col = fxSmoke(ro, rd, y0, y1, 2.6, 3.0, 2.7, AR_GOLD)*(0.5 + 0.3*sw);
  col += fxSmoke(ro, rd, y0 + 3.0, y1, 1.6, 4.0, 6.1, AR_PALE)*0.3;
  col += fxAura(ro, rd, y0, y1, 2.4, AR_GOLD)*0.1;
  return col*uFx;
}

// the blade: Sol's glow and core from the file, white-gold, with slow waves of light rising
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float wave = pow(0.5 + 0.5*sin(b.y*0.25 - uTime*1.1), 3.0);
  return recol(g, mix(AR_GOLD, AR_PALE, 0.5 + 0.5*wave))*(0.72 + 0.28*wave + 0.3*clamp(uSwing, 0.0, 1.0));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c*(1.0 + 0.5*clamp(uClash, 0.0, 1.0)); }

// ---- round 9: the big effects layer ----
// Round 11 (Creyon): its own look, Sol's way: its round-8 effects as part 0, its own parts
// (`part(k)`, the `// @part` lines) and Sol's pieces from its file; nothing shared with the others.
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 col = r8_vol(ro, rd, px)*part(0.0);
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  fx += fxStreams(ro, rd, px, y0, y1, 3.0, 2.4, 0.16, 1.4, 4.2, vec3(1.0, 0.84, 0.5))*part(1.0);
  // round 14 (Creyon): a chroma for the blade only; its effects keep their own colours
  uKeep += col + fx*uFx;
  return vec3(0.0);
}
