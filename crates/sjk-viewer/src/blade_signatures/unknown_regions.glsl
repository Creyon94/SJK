// Unknown Regions: the uncharted dark beyond the galaxy's edge. A deep violet blade with a shadowed
// heart, nebula swirling slowly inside it and round it in soft violet clouds, faint stars drifting
// through them and a far spiral galaxy turning slowly beside it (round 15: it replaced threads of
// starlight it shared with the Aurodium). A clash flares it.
// @part Nebula
// @part Drifting stars
// @part Far galaxy
// @room 8
// @palette {"corona.rim_cool":[0.15,0.05,0.4],"corona.rim_hot":[0.6,0.3,1.0],"corona.inner":[0.12,0.04,0.3],"core.white":[0.45,0.3,0.8],"core.fringe_cool":[0.35,0.15,0.9],"core.fringe_hot":[0.7,0.5,1.0],"light.color":[0.5,0.3,1.0],"trail":[0.5,0.3,1.0]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float n = fbm3(vec2(b.x*1.2 + uTime*0.1, b.y*0.25 - uTime*0.15));
  float heart = 1.0 - 0.4*exp(-pow(b.x/0.25, 2.0));
  return g*heart*(0.6 + 0.8*n);
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 v = vec3(0.5, 0.3, 1.0), w = vec3(0.85, 0.85, 1.0);
  fx += fxSmoke(ro, rd, y0, y1, 4, 0.6, 8.8, v)*1.1*part(0.0);
  fx += fxMotes(ro, rd, px, y0, y1, 0.8, 4.5, 28.0, 0.015, 3.9, w)*0.8*part(1.0);
  // the far galaxy: two spiral arms round a bright heart, tilted, turning slowly
  float ta;
  vec3 gc = vec3(-3.4, L*0.7, 0.0);
  float dg = rayPt(ro, rd, gc, ta);
  if (dg < 4.0){
    vec3 q = ro + rd*ta - gc;
    vec2 e = rot(t*0.12)*vec2(q.x, q.y*1.8)/1.5;
    float r = length(e);
    float an = atan(e.y, e.x);
    float arms = pow(0.5 + 0.5*cos(2.0*an - 4.2*log(max(r, 0.05))), 3.0);
    float g = arms*exp(-r*1.3)*smoothstep(0.05, 0.3, r) + exp(-r*r*18.0)*1.4;
    fx += (mix(v, w, exp(-r*2.0))*g*2.2 + w*halo(dg, max(0.07, px))*1.2)*part(2.0);
  }
  return fx*uFx;
}
