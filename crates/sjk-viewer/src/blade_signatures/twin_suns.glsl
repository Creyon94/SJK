// Twin Suns: Tatooine's two suns at full blaze. A blade of gold and orange halves rolling into each
// other with a seething sun's surface, loops of solar plasma arching off it and snapping back, a
// solar wind of sparks streaming off it and the air shimmering with heat; round 16 (Creyon: it was
// missing something about the suns, and Sol's Sun blade has two suns already): the light of a
// Tatooine sunset, broad golden sunbeams slanting across it from one side, slowly sweeping, with
// dust glittering in them. A clash flares it.
// @part Sun glow
// @part Solar wind
// @part Heat
// @arcs Solar prominences
// @room 8
// @palette {"corona.rim_cool":[0.6,0.2,0.02],"corona.rim_hot":[1.0,0.6,0.15],"corona.inner":[1.0,0.9,0.6],"core.white":[1,0.97,0.85],"core.fringe_cool":[1.0,0.5,0.1],"core.fringe_hot":[1.0,0.85,0.5],"light.color":[1.0,0.62,0.2],"trail":[1.0,0.62,0.2]}

vec3 sig_glow_own(vec3 g, vec2 b, float r, float px){
  vec3 a = vec3(1.0, 0.85, 0.4), o = vec3(1.0, 0.45, 0.1);
  float boil = fbm3(vec2(b.x*3.0, b.y*0.6 - uTime*0.7))*part(0.0);
  return recol(g, mix(a, o, bands(b.y, 0.05, 0.3)))*(0.95 + 0.4*boil);
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 gold = vec3(1.0, 0.7, 0.25);
  fx += fxAura(ro, rd, y0, y1, 2.6, gold)*0.5*part(0.0);
  // sunbeams: broad soft shafts of low golden light slanting across the blade from one side,
  // sweeping slowly, dust glittering in them
  if (fxNear(ro, rd, y0, y1) < 7.0 && abs(rd.z) > 0.0001){
    vec3 p = ro + rd*(-ro.z/rd.z);
    float slant = 0.55 + 0.08*sin(t*0.15);
    float u = p.y*cos(slant) + p.x*sin(slant);
    float shafts = pow(0.5 + 0.5*sin(u*0.55 - t*0.3), 3.0)*(0.6 + 0.4*sin(u*0.17 + t*0.11));
    float reach = exp(-pow(abs(p.x)/5.5, 2.0))*smoothstep(y0 - 3.0, y0 + 2.0, p.y)*(1.0 - smoothstep(y1 - 4.0, y1 + 2.0, p.y));
    float fromSide = smoothstep(-6.0, 1.0, -p.x);
    fx += vec3(1.0, 0.72, 0.32)*shafts*reach*(0.35 + 0.65*fromSide)*0.75*part(0.0);
    float dust = step(0.985, h21(floor(p.xy*vec2(6.0, 3.0) + vec2(0.0, -t*2.0))))*shafts*reach;
    fx += vec3(1.0, 0.9, 0.6)*dust*0.9*part(0.0);
  }
  fx += fxMotes(ro, rd, px, y0, y1, 1.0, 5.0, 24.0, 0.12, 2.4, vec3(1.0, 0.85, 0.5))*part(1.0);
  fx += fxSmoke(ro, rd, y0, y1, 2.8, 3.0, 5.0, vec3(1.0, 0.45, 0.1))*0.35*part(2.0);
  return fx*uFx;
}

// round 15 (Creyon: some blades glowed so much their effects hardly showed): its glow at
// 80 %, so what is drawn round it reads
vec3 sig_glow(vec3 g, vec2 b, float r, float px){ return sig_glow_own(g, b, r, px)*0.8; }
