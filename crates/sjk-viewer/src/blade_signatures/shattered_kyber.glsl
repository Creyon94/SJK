// Shattered Kyber: a cracked kyber crystal that will not stay whole. Fine cracks flicker with
// light through the blade; when it swings it breaks into clear glass shards that float round it,
// and when it rests they snap back; light leaks out round it unevenly and fine kyber dust
// glitters off it. A clash shatters it for a moment.
// @part Shards
// @part Crack light
// @part Kyber dust
// @arcs Crack lightning
// @room 8
// @solid 1
// @palette {"corona.rim_cool":[0.3,0.4,0.8],"corona.rim_hot":[0.75,0.85,1.0],"corona.inner":[0.95,0.98,1.0],"core.white":[1,1,1],"core.fringe_cool":[0.6,0.75,1.0],"core.fringe_hot":[0.9,0.95,1.0],"light.color":[0.7,0.82,1.0],"trail":[0.7,0.82,1.0]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  vec2 cell;
  float e = voro(vec2(b.x*2.5, b.y*0.6), cell);
  float fl = 0.6 + 0.4*sin(uTime*5.0 + h21(cell)*20.0)*part(1.0);
  return g*(0.85 + 0.9*halo(e, 0.05)*fl);
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 c = vec3(0.75, 0.88, 1.0);
  float leak = 0.6 + 0.4*sin(t*2.3)*sin(t*1.1);
  fx += fxAura(ro, rd, y0, y1, 2.0, c)*0.5*leak*part(1.0);
  fx += fxMotes(ro, rd, px, y0, y1, 0.8, 3.5, 20.0, 0.04, 9.2, c)*part(2.0);
  fx += fxSparks(ro, rd, px, y0, y1, 4.0 + 10.0*sw, 3.0, 0.6, 5.1, vec3(0.8, 0.9, 1.0))*part(2.0);
  return fx*uFx;
}
// only what circles the blade is drawn in 3D
vec4 sig_orbit_solid(vec3 ro, vec3 rd, float px){
  vec4 col = vec4(0.0);
  float out_ = max(sqrt(clamp(uSwing, 0.0, 1.0)), clamp(uClash, 0.0, 1.0));
  if (uFx < 0.01 || out_ < 0.02 || part(0.0) < 0.5) return col;
  float best = 1e5, th;
  for (int i = 0; i < 16; i++){
    float fi = float(i);
    vec3 c = around(0.5 + 3.5*out_*(0.5 + 0.5*h11(fi*5.1)), h11(fi*1.7) + uTime*0.05, (0.08 + 0.9*fract(fi*0.381 + 0.07))*uLength);
    float d = rayPt(ro, rd, c, th);
    vec3 q = ro + rd*th - c;
    vec2 a = rot(uTime*0.6 + fi)*q.xy;
    float k = max(abs(a.x)*1.6 + abs(a.y)*0.5, 0.0) - 0.45*out_;
    float cv = clamp(0.5 - k/max(px, 1e-3), 0.0, 1.0)*step(d, 1.5);
    if (cv > 0.0 && th < best){
      best = th;
      col = vec4(vec3(0.75, 0.88, 1.0)*(0.6 + 0.6*halo(abs(k), 0.06)) + vec3(0.2, 0.25, 0.35), cv*0.85);
    }
  }
  return col;
}
