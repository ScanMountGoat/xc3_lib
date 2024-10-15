const int undef = 0;
layout(binding = 0, std140) uniform _support_buffer {
    uint alpha_test;
    uint is_bgra[8];
    precise vec4 viewport_inverse;
    precise vec4 viewport_size;
    int frag_scale_count;
    precise float render_scale[73];
    ivec4 tfe_offset;
    int tfe_vertex_count;
}support_buffer;
layout(binding = 7, std140) uniform _U_VolTexCalc {
    vec4 gVolTexCalcWork[10];
}U_VolTexCalc;
layout(binding = 8, std140) uniform _U_RimBloomCalc {
    vec4 gRimBloomCalcWork[2];
}U_RimBloomCalc;
layout(binding = 2, std140) uniform _fp_c1 {
    precise vec4 data[4096];
}fp_c1;
layout(binding = 0) uniform sampler3D volTex0;
layout(binding = 1) uniform sampler2D s1;
layout(binding = 2) uniform sampler2D s2;
layout(binding = 3) uniform sampler2D s4;
layout(binding = 4) uniform sampler2D s0;
layout(binding = 5) uniform sampler2D s5;
layout(binding = 6) uniform sampler2D s3;
layout(location = 0) in vec4 in_attr0;
layout(location = 1) in vec4 in_attr1;
layout(location = 2) in vec4 in_attr2;
layout(location = 3) in vec4 in_attr3;
layout(location = 4) in vec4 in_attr4;
layout(location = 5) in vec4 in_attr5;
layout(location = 6) in vec4 in_attr6;
layout(location = 7) in vec4 in_attr7;
layout(location = 0) out vec4 out_attr0;
layout(location = 1) out vec4 out_attr1;
layout(location = 2) out vec4 out_attr2;
layout(location = 3) out vec4 out_attr3;
layout(location = 4) out vec4 out_attr4;
void main() {
    precise float temp_0;
    precise float temp_1;
    precise float temp_2;
    precise float temp_3;
    precise float temp_4;
    precise float temp_5;
    precise float temp_6;
    precise float temp_7;
    precise float temp_8;
    precise float temp_9;
    precise float temp_10;
    precise float temp_11;
    precise float temp_12;
    precise float temp_13;
    precise float temp_14;
    precise float temp_15;
    precise float temp_16;
    precise float temp_17;
    precise float temp_18;
    precise float temp_19;
    precise float temp_20;
    precise float temp_21;
    precise float temp_22;
    precise float temp_23;
    precise float temp_24;
    precise float temp_25;
    precise float temp_26;
    precise float temp_27;
    precise float temp_28;
    precise float temp_29;
    bool temp_30;
    precise vec2 temp_31;
    precise float temp_32;
    precise float temp_33;
    precise vec2 temp_34;
    precise float temp_35;
    precise float temp_36;
    precise float temp_37;
    precise vec3 temp_38;
    precise float temp_39;
    precise float temp_40;
    precise float temp_41;
    precise float temp_42;
    precise float temp_43;
    precise float temp_44;
    precise float temp_45;
    precise float temp_46;
    precise float temp_47;
    precise float temp_48;
    precise float temp_49;
    precise float temp_50;
    precise float temp_51;
    precise float temp_52;
    precise float temp_53;
    bool temp_54;
    precise float temp_55;
    precise float temp_56;
    precise float temp_57;
    precise float temp_58;
    precise float temp_59;
    precise float temp_60;
    precise float temp_61;
    precise float temp_62;
    precise float temp_63;
    precise float temp_64;
    precise float temp_65;
    precise float temp_66;
    precise float temp_67;
    precise float temp_68;
    precise float temp_69;
    precise float temp_70;
    precise float temp_71;
    precise float temp_72;
    precise float temp_73;
    precise float temp_74;
    precise float temp_75;
    precise float temp_76;
    precise float temp_77;
    precise float temp_78;
    precise float temp_79;
    precise float temp_80;
    precise float temp_81;
    precise float temp_82;
    precise float temp_83;
    precise float temp_84;
    precise float temp_85;
    precise float temp_86;
    precise float temp_87;
    precise float temp_88;
    precise float temp_89;
    precise float temp_90;
    precise float temp_91;
    precise float temp_92;
    precise float temp_93;
    precise float temp_94;
    precise float temp_95;
    precise float temp_96;
    precise float temp_97;
    precise float temp_98;
    precise float temp_99;
    precise float temp_100;
    precise float temp_101;
    bool temp_102;
    precise float temp_103;
    precise float temp_104;
    precise float temp_105;
    int temp_106;
    precise float temp_107;
    precise float temp_108;
    precise float temp_109;
    bool temp_110;
    precise float temp_111;
    precise float temp_112;
    precise float temp_113;
    precise float temp_114;
    precise float temp_115;
    precise float temp_116;
    precise float temp_117;
    precise float temp_118;
    precise float temp_119;
    precise float temp_120;
    precise float temp_121;
    precise float temp_122;
    precise float temp_123;
    precise float temp_124;
    precise float temp_125;
    precise float temp_126;
    precise float temp_127;
    precise float temp_128;
    precise float temp_129;
    precise float temp_130;
    precise float temp_131;
    precise float temp_132;
    precise float temp_133;
    precise float temp_134;
    precise float temp_135;
    precise float temp_136;
    precise float temp_137;
    precise float temp_138;
    precise float temp_139;
    precise float temp_140;
    precise float temp_141;
    precise float temp_142;
    precise float temp_143;
    precise float temp_144;
    precise float temp_145;
    precise float temp_146;
    precise float temp_147;
    precise float temp_148;
    precise float temp_149;
    precise float temp_150;
    precise float temp_151;
    precise float temp_152;
    precise float temp_153;
    precise float temp_154;
    precise float temp_155;
    precise float temp_156;
    precise float temp_157;
    precise float temp_158;
    precise float temp_159;
    precise float temp_160;
    precise float temp_161;
    precise float temp_162;
    precise float temp_163;
    precise float temp_164;
    precise float temp_165;
    precise float temp_166;
    precise float temp_167;
    precise float temp_168;
    precise float temp_169;
    precise float temp_170;
    precise float temp_171;
    precise float temp_172;
    precise float temp_173;
    precise float temp_174;
    precise float temp_175;
    precise float temp_176;
    precise float temp_177;
    precise float temp_178;
    precise float temp_179;
    precise float temp_180;
    precise float temp_181;
    precise float temp_182;
    precise float temp_183;
    precise float temp_184;
    precise float temp_185;
    precise float temp_186;
    precise float temp_187;
    precise float temp_188;
    precise float temp_189;
    precise float temp_190;
    precise float temp_191;
    precise float temp_192;
    precise float temp_193;
    precise float temp_194;
    precise float temp_195;
    precise float temp_196;
    precise float temp_197;
    precise float temp_198;
    precise float temp_199;
    precise float temp_200;
    precise float temp_201;
    int temp_202;
    precise float temp_203;
    precise float temp_204;
    int temp_205;
    precise float temp_206;
    precise float temp_207;
    precise float temp_208;
    bool temp_209;
    precise float temp_210;
    precise float temp_211;
    precise float temp_212;
    int temp_213;
    precise float temp_214;
    precise float temp_215;
    int temp_216;
    precise float temp_217;
    precise float temp_218;
    precise float temp_219;
    bool temp_220;
    bool temp_221;
    precise float temp_222;
    precise float temp_223;
    precise float temp_224;
    int temp_225;
    int temp_226;
    int temp_227;
    precise float temp_228;
    precise float temp_229;
    precise float temp_230;
    int temp_231;
    precise float temp_232;
    precise float temp_233;
    precise float temp_234;
    int temp_235;
    int temp_236;
    int temp_237;
    int temp_238;
    precise float temp_239;
    int temp_240;
    int temp_241;
    int temp_242;
    int temp_243;
    int temp_244;
    int temp_245;
    int temp_246;
    int temp_247;
    int temp_248;
    int temp_249;
    int temp_250;
    int temp_251;
    precise float temp_252;
    bool temp_253;
    precise float temp_254;
    precise float temp_255;
    precise float temp_256;
    precise float temp_257;
    precise float temp_258;
    precise float temp_259;
    precise float temp_260;
    precise float temp_261;
    precise float temp_262;
    precise float temp_263;
    precise float temp_264;
    precise float temp_265;
    precise float temp_266;
    precise float temp_267;
    precise float temp_268;
    precise float temp_269;
    precise float temp_270;
    precise float temp_271;
    bool temp_272;
    int temp_273;
    int temp_274;
    int temp_275;
    int temp_276;
    bool temp_277;
    int temp_278;
    int temp_279;
    int temp_280;
    int temp_281;
    precise float temp_282;
    precise float temp_283;
    precise float temp_284;
    bool temp_285;
    bool temp_286;
    bool temp_287;
    int temp_288;
    int temp_289;
    int temp_290;
    int temp_291;
    int temp_292;
    int temp_293;
    int temp_294;
    int temp_295;
    precise float temp_296;
    precise float temp_297;
    precise float temp_298;
    bool temp_299;
    precise float temp_300;
    precise float temp_301;
    precise float temp_302;
    precise float temp_303;
    precise float temp_304;
    precise float temp_305;
    precise float temp_306;
    precise float temp_307;
    precise float temp_308;
    precise float temp_309;
    precise float temp_310;
    precise float temp_311;
    precise float temp_312;
    precise float temp_313;
    precise float temp_314;
    precise float temp_315;
    precise float temp_316;
    precise float temp_317;
    bool temp_318;
    int temp_319;
    int temp_320;
    int temp_321;
    int temp_322;
    bool temp_323;
    int temp_324;
    int temp_325;
    int temp_326;
    int temp_327;
    precise float temp_328;
    precise float temp_329;
    precise float temp_330;
    bool temp_331;
    bool temp_332;
    bool temp_333;
    int temp_334;
    int temp_335;
    int temp_336;
    int temp_337;
    int temp_338;
    int temp_339;
    int temp_340;
    int temp_341;
    precise float temp_342;
    precise float temp_343;
    precise float temp_344;
    bool temp_345;
    precise float temp_346;
    precise float temp_347;
    precise float temp_348;
    precise float temp_349;
    precise float temp_350;
    precise float temp_351;
    precise float temp_352;
    precise float temp_353;
    precise float temp_354;
    precise float temp_355;
    precise float temp_356;
    precise float temp_357;
    precise float temp_358;
    precise float temp_359;
    precise float temp_360;
    precise float temp_361;
    precise float temp_362;
    precise float temp_363;
    bool temp_364;
    int temp_365;
    int temp_366;
    int temp_367;
    int temp_368;
    bool temp_369;
    int temp_370;
    int temp_371;
    int temp_372;
    int temp_373;
    precise float temp_374;
    precise float temp_375;
    precise float temp_376;
    bool temp_377;
    bool temp_378;
    bool temp_379;
    int temp_380;
    int temp_381;
    int temp_382;
    int temp_383;
    int temp_384;
    int temp_385;
    int temp_386;
    int temp_387;
    precise float temp_388;
    precise float temp_389;
    precise float temp_390;
    bool temp_391;
    precise float temp_392;
    precise float temp_393;
    precise float temp_394;
    precise float temp_395;
    precise float temp_396;
    precise float temp_397;
    precise float temp_398;
    precise float temp_399;
    precise float temp_400;
    precise float temp_401;
    precise float temp_402;
    precise float temp_403;
    precise float temp_404;
    precise float temp_405;
    precise float temp_406;
    precise float temp_407;
    precise float temp_408;
    precise float temp_409;
    bool temp_410;
    int temp_411;
    int temp_412;
    int temp_413;
    int temp_414;
    bool temp_415;
    int temp_416;
    int temp_417;
    int temp_418;
    int temp_419;
    precise float temp_420;
    precise float temp_421;
    precise float temp_422;
    bool temp_423;
    bool temp_424;
    int temp_425;
    int temp_426;
    int temp_427;
    int temp_428;
    precise float temp_429;
    precise float temp_430;
    bool temp_431;
    precise float temp_432;
    precise float temp_433;
    precise float temp_434;
    precise float temp_435;
    precise float temp_436;
    precise float temp_437;
    precise float temp_438;
    precise float temp_439;
    precise float temp_440;
    precise float temp_441;
    precise float temp_442;
    precise float temp_443;
    precise float temp_444;
    precise float temp_445;
    precise float temp_446;
    precise float temp_447;
    precise float temp_448;
    precise float temp_449;
    int temp_450;
    int temp_451;
    int temp_452;
    int temp_453;
    precise float temp_454;
    precise float temp_455;
    precise float temp_456;
    precise float temp_457;
    precise float temp_458;
    precise float temp_459;
    precise float temp_460;
    precise float temp_461;
    precise float temp_462;
    precise float temp_463;
    precise float temp_464;
    precise float temp_465;
    bool temp_466;
    precise float temp_467;
    precise float temp_468;
    bool temp_469;
    precise float temp_470;
    precise float temp_471;
    precise float temp_472;
    precise float temp_473;
    bool temp_474;
    precise float temp_475;
    precise float temp_476;
    precise float temp_477;
    precise float temp_478;
    precise float temp_479;
    bool temp_480;
    precise float temp_481;
    precise float temp_482;
    precise float temp_483;
    precise float temp_484;
    precise float temp_485;
    precise float temp_486;
    precise float temp_487;
    precise float temp_488;
    precise float temp_489;
    precise float temp_490;
    temp_0 = in_attr7.z;
    temp_1 = in_attr7.x;
    temp_2 = in_attr7.y;
    temp_3 = temp_0 * U_VolTexCalc.gVolTexCalcWork[0].z;
    temp_4 = temp_1 * U_VolTexCalc.gVolTexCalcWork[0].x;
    temp_5 = temp_2 * U_VolTexCalc.gVolTexCalcWork[0].y;
    temp_6 = texture(volTex0, vec3(temp_4, temp_5, temp_3)).x;
    temp_7 = 0. - U_VolTexCalc.gVolTexCalcWork[1].x;
    temp_8 = temp_1 + temp_7;
    temp_9 = in_attr4.x;
    temp_10 = 0. - U_VolTexCalc.gVolTexCalcWork[1].y;
    temp_11 = temp_2 + temp_10;
    temp_12 = in_attr4.y;
    temp_13 = temp_8 * temp_8;
    temp_14 = in_attr4.z;
    temp_15 = 0. - U_VolTexCalc.gVolTexCalcWork[1].z;
    temp_16 = temp_0 + temp_15;
    temp_17 = in_attr4.w;
    temp_18 = fma(temp_11, temp_11, temp_13);
    temp_19 = fma(temp_16, temp_16, temp_18);
    temp_20 = sqrt(temp_19);
    temp_21 = 1. / U_VolTexCalc.gVolTexCalcWork[4].z;
    temp_22 = U_VolTexCalc.gVolTexCalcWork[4].w + U_VolTexCalc.gVolTexCalcWork[1].w;
    temp_23 = min(temp_20, U_VolTexCalc.gVolTexCalcWork[4].z);
    temp_24 = fma(temp_22, U_VolTexCalc.gVolTexCalcWork[0].w, U_VolTexCalc.gVolTexCalcWork[0].w);
    temp_25 = temp_23 * temp_21;
    temp_26 = 0. - U_VolTexCalc.gVolTexCalcWork[4].w;
    temp_27 = fma(temp_25, temp_26, temp_24);
    temp_28 = 0. - U_VolTexCalc.gVolTexCalcWork[1].w;
    temp_29 = temp_27 + temp_28;
    temp_30 = temp_6 < temp_29;
    if (temp_30) {
        discard;
    }
    temp_31 = texture(s1, vec2(temp_9, temp_12)).xy;
    temp_32 = temp_31.x;
    temp_33 = temp_31.y;
    temp_34 = texture(s2, vec2(temp_9, temp_12)).xy;
    temp_35 = temp_34.x;
    temp_36 = temp_34.y;
    temp_37 = texture(s4, vec2(temp_14, temp_17)).y;
    temp_38 = texture(s0, vec2(temp_9, temp_12)).xyz;
    temp_39 = temp_38.x;
    temp_40 = temp_38.y;
    temp_41 = temp_38.z;
    temp_42 = texture(s5, vec2(temp_9, temp_12)).x;
    temp_43 = texture(s3, vec2(temp_9, temp_12)).z;
    temp_44 = in_attr1.x;
    temp_45 = in_attr1.y;
    temp_46 = in_attr1.z;
    temp_47 = in_attr0.x;
    temp_48 = in_attr0.y;
    temp_49 = temp_44 * temp_44;
    temp_50 = in_attr6.w;
    temp_51 = fma(temp_45, temp_45, temp_49);
    temp_52 = fma(temp_46, temp_46, temp_51);
    temp_53 = in_attr0.z;
    temp_54 = 0. < U_RimBloomCalc.gRimBloomCalcWork[1].z;
    temp_55 = inversesqrt(temp_52);
    temp_56 = temp_44 * temp_55;
    temp_57 = temp_45 * temp_55;
    temp_58 = temp_46 * temp_55;
    temp_59 = temp_47 * temp_47;
    temp_60 = fma(temp_48, temp_48, temp_59);
    temp_61 = fma(temp_53, temp_53, temp_60);
    temp_62 = inversesqrt(temp_61);
    temp_63 = temp_47 * temp_62;
    temp_64 = temp_48 * temp_62;
    temp_65 = temp_53 * temp_62;
    temp_66 = fma(temp_32, 2., -1.);
    temp_67 = fma(temp_33, 2., -1.);
    temp_68 = temp_66 * temp_56;
    temp_69 = temp_66 * temp_57;
    temp_70 = temp_66 * temp_58;
    temp_71 = temp_66 * temp_66;
    temp_72 = fma(temp_67, temp_67, temp_71);
    temp_73 = 0. - temp_72;
    temp_74 = temp_73 + 1.;
    temp_75 = in_attr2.x;
    temp_76 = sqrt(temp_74);
    temp_77 = max(0., temp_76);
    temp_78 = fma(temp_63, temp_77, temp_68);
    temp_79 = in_attr2.y;
    temp_80 = fma(temp_64, temp_77, temp_69);
    temp_81 = in_attr2.z;
    temp_82 = fma(temp_65, temp_77, temp_70);
    temp_83 = 1. / temp_50;
    temp_84 = temp_75 * temp_75;
    temp_85 = fma(temp_79, temp_79, temp_84);
    temp_86 = intBitsToFloat(gl_FrontFacing ? -1 : 0);
    temp_87 = fma(temp_81, temp_81, temp_85);
    temp_88 = inversesqrt(temp_87);
    temp_89 = temp_75 * temp_88;
    temp_90 = temp_79 * temp_88;
    temp_91 = float(floatBitsToInt(temp_86));
    temp_92 = fma(temp_67, temp_89, temp_78);
    temp_93 = temp_81 * temp_88;
    temp_94 = in_attr6.x;
    temp_95 = fma(temp_67, temp_90, temp_80);
    temp_96 = temp_92 * temp_92;
    temp_97 = fma(temp_67, temp_93, temp_82);
    temp_98 = fma(temp_95, temp_95, temp_96);
    temp_99 = fma(temp_91, -2., -1.);
    temp_100 = fma(temp_97, temp_97, temp_98);
    temp_101 = inversesqrt(temp_100);
    temp_102 = floatBitsToInt(temp_99) > 0;
    temp_103 = in_attr6.y;
    temp_104 = temp_36 * temp_36;
    temp_105 = temp_92 * temp_101;
    temp_106 = 0 - (temp_102 ? -1 : 0);
    temp_107 = temp_36 * temp_104;
    temp_108 = temp_95 * temp_101;
    temp_109 = in_attr3.x;
    temp_110 = temp_106 == 0;
    temp_111 = temp_97 * temp_101;
    temp_112 = fma(temp_36, -2., 2.);
    temp_113 = temp_36 * temp_107;
    temp_114 = clamp(temp_113, 0., 1.);
    temp_115 = in_attr3.y;
    temp_116 = temp_36 * 2.;
    temp_117 = temp_116 * temp_37;
    temp_118 = 0. - temp_112;
    temp_119 = fma(temp_37, temp_118, temp_112);
    temp_120 = 0. - temp_114;
    temp_121 = fma(temp_119, temp_120, temp_114);
    temp_122 = 0. - temp_117;
    temp_123 = fma(temp_114, temp_122, temp_117);
    temp_124 = in_attr3.z;
    temp_125 = temp_109 * temp_109;
    temp_126 = fma(temp_115, temp_115, temp_125);
    temp_127 = fma(temp_124, temp_124, temp_126);
    temp_128 = in_attr5.w;
    temp_129 = temp_94 * temp_83;
    temp_130 = inversesqrt(temp_127);
    temp_131 = temp_103 * temp_83;
    temp_132 = in_attr5.x;
    temp_133 = temp_105;
    temp_134 = temp_108;
    temp_135 = temp_111;
    temp_136 = temp_115;
    temp_137 = temp_132;
    temp_138 = temp_129;
    temp_139 = temp_128;
    if (temp_110) {
        temp_140 = 0. - temp_105;
        temp_141 = temp_140 + -0.;
        temp_133 = temp_141;
    }
    temp_142 = temp_133;
    if (temp_110) {
        temp_143 = 0. - temp_108;
        temp_144 = temp_143 + -0.;
        temp_134 = temp_144;
    }
    temp_145 = temp_134;
    temp_146 = temp_109 * temp_130;
    temp_147 = temp_115 * temp_130;
    temp_148 = temp_124 * temp_130;
    temp_149 = temp_146;
    temp_150 = temp_148;
    if (temp_54) {
        temp_151 = 0. - temp_142;
        temp_152 = temp_146 * temp_151;
        temp_149 = temp_152;
    }
    temp_153 = temp_149;
    temp_154 = temp_153;
    if (temp_110) {
        temp_155 = 0. - temp_111;
        temp_156 = temp_155 + -0.;
        temp_135 = temp_156;
    }
    temp_157 = temp_135;
    temp_158 = 1. / temp_128;
    if (temp_54) {
        temp_159 = 0. - temp_145;
        temp_160 = fma(temp_147, temp_159, temp_153);
        temp_136 = temp_160;
    }
    temp_161 = temp_136;
    temp_162 = temp_161;
    if (temp_54) {
        temp_163 = 0. - temp_157;
        temp_164 = fma(temp_148, temp_163, temp_161);
        temp_162 = temp_164;
    }
    temp_165 = temp_162;
    temp_166 = 0. - temp_129;
    temp_167 = fma(temp_132, temp_158, temp_166);
    if (temp_54) {
        temp_168 = abs(temp_165);
        temp_169 = 0. - temp_168;
        temp_170 = temp_169 + 1.;
        temp_137 = temp_170;
    }
    temp_171 = temp_137;
    temp_172 = in_attr5.y;
    temp_173 = temp_171;
    temp_174 = temp_172;
    if (temp_54) {
        temp_150 = U_RimBloomCalc.gRimBloomCalcWork[1].x;
    }
    temp_175 = temp_150;
    temp_176 = temp_175;
    if (temp_54) {
        temp_177 = log2(temp_171);
        temp_173 = temp_177;
    }
    temp_178 = temp_173;
    temp_179 = temp_178;
    if (temp_54) {
        temp_180 = temp_175 * 10.;
        temp_176 = temp_180;
    }
    temp_181 = temp_176;
    temp_182 = temp_181;
    if (temp_54) {
        temp_183 = temp_181 * temp_178;
        temp_182 = temp_183;
    }
    temp_184 = temp_182;
    temp_185 = temp_184;
    if (temp_54) {
        temp_154 = temp_184;
    }
    temp_186 = temp_154;
    if (temp_54) {
        temp_187 = exp2(temp_186);
        temp_138 = temp_187;
    }
    temp_188 = temp_138;
    temp_189 = 0. - temp_131;
    temp_190 = fma(temp_172, temp_158, temp_189);
    temp_191 = temp_188;
    if (temp_54) {
        temp_192 = 0. - U_RimBloomCalc.gRimBloomCalcWork[1].y;
        temp_193 = fma(temp_39, temp_192, temp_39);
        temp_174 = temp_193;
    }
    temp_194 = temp_174;
    if (temp_54) {
        temp_195 = temp_188 * U_RimBloomCalc.gRimBloomCalcWork[1].z;
        temp_191 = temp_195;
    }
    temp_196 = temp_191;
    if (temp_54) {
        temp_197 = 0. - temp_194;
        temp_198 = temp_197 + U_RimBloomCalc.gRimBloomCalcWork[0].x;
        temp_185 = temp_198;
    }
    temp_199 = temp_185;
    temp_200 = temp_121 + temp_123;
    temp_201 = temp_29 + U_VolTexCalc.gVolTexCalcWork[4].y;
    temp_202 = floatBitsToInt(temp_39);
    temp_203 = temp_199;
    if (temp_54) {
        temp_204 = fma(temp_199, temp_196, temp_194);
        temp_202 = floatBitsToInt(temp_204);
    }
    temp_205 = temp_202;
    if (temp_54) {
        temp_206 = 0. - U_RimBloomCalc.gRimBloomCalcWork[1].y;
        temp_207 = fma(temp_40, temp_206, temp_40);
        temp_203 = temp_207;
    }
    temp_208 = temp_203;
    temp_209 = temp_6 < temp_201;
    if (temp_54) {
        temp_210 = 0. - temp_208;
        temp_211 = temp_210 + U_RimBloomCalc.gRimBloomCalcWork[0].y;
        temp_139 = temp_211;
    }
    temp_212 = temp_139;
    temp_213 = floatBitsToInt(temp_40);
    temp_214 = temp_212;
    if (temp_54) {
        temp_215 = fma(temp_212, temp_196, temp_208);
        temp_213 = floatBitsToInt(temp_215);
    }
    temp_216 = temp_213;
    if (temp_54) {
        temp_217 = 0. - U_RimBloomCalc.gRimBloomCalcWork[1].y;
        temp_218 = fma(temp_41, temp_217, temp_41);
        temp_214 = temp_218;
    }
    temp_219 = temp_214;
    temp_220 = !temp_209;
    temp_221 = temp_220;
    if (temp_54) {
        temp_222 = 0. - temp_219;
        temp_223 = temp_222 + U_RimBloomCalc.gRimBloomCalcWork[0].z;
        temp_179 = temp_223;
    }
    temp_224 = temp_179;
    temp_225 = 0;
    temp_226 = floatBitsToInt(temp_41);
    if (temp_54) {
        temp_225 = floatBitsToInt(temp_196);
    }
    temp_227 = temp_225;
    temp_228 = 0. - temp_42;
    temp_229 = fma(temp_36, temp_228, temp_36);
    if (temp_54) {
        temp_230 = fma(temp_224, temp_196, temp_219);
        temp_226 = floatBitsToInt(temp_230);
    }
    temp_231 = temp_226;
    temp_232 = temp_29;
    if (temp_220) {
        temp_232 = temp_201;
    }
    temp_233 = temp_232;
    temp_234 = fma(temp_200, temp_42, temp_229);
    temp_235 = temp_227;
    temp_236 = temp_205;
    temp_237 = temp_216;
    temp_238 = temp_231;
    temp_239 = temp_233;
    if (temp_209) {
        temp_235 = floatBitsToInt(U_VolTexCalc.gVolTexCalcWork[9].w);
    }
    temp_240 = temp_235;
    temp_241 = temp_240;
    temp_242 = temp_240;
    if (temp_209) {
        temp_236 = floatBitsToInt(U_VolTexCalc.gVolTexCalcWork[9].x);
    }
    temp_243 = temp_236;
    temp_244 = temp_243;
    temp_245 = temp_243;
    if (temp_209) {
        temp_237 = floatBitsToInt(U_VolTexCalc.gVolTexCalcWork[9].y);
    }
    temp_246 = temp_237;
    temp_247 = temp_246;
    temp_248 = temp_246;
    if (temp_209) {
        temp_238 = floatBitsToInt(U_VolTexCalc.gVolTexCalcWork[9].z);
    }
    temp_249 = temp_238;
    temp_250 = temp_249;
    temp_251 = temp_249;
    if (temp_220) {
        temp_252 = temp_233 + U_VolTexCalc.gVolTexCalcWork[4].x;
        temp_253 = temp_6 < temp_252;
        if (temp_253) {
            temp_254 = 0. - temp_233;
            temp_255 = temp_254 + temp_252;
            temp_256 = 1. / temp_255;
            temp_257 = 0. - temp_233;
            temp_258 = temp_257 + temp_6;
            temp_259 = 0. - U_VolTexCalc.gVolTexCalcWork[9].x;
            temp_260 = temp_259 + U_VolTexCalc.gVolTexCalcWork[8].x;
            temp_261 = 0. - U_VolTexCalc.gVolTexCalcWork[9].y;
            temp_262 = temp_261 + U_VolTexCalc.gVolTexCalcWork[8].y;
            temp_263 = 0. - U_VolTexCalc.gVolTexCalcWork[9].z;
            temp_264 = temp_263 + U_VolTexCalc.gVolTexCalcWork[8].z;
            temp_265 = temp_258 * temp_256;
            temp_266 = 0. - U_VolTexCalc.gVolTexCalcWork[9].w;
            temp_267 = U_VolTexCalc.gVolTexCalcWork[8].w + temp_266;
            temp_268 = fma(temp_260, temp_265, U_VolTexCalc.gVolTexCalcWork[9].x);
            temp_269 = fma(temp_262, temp_265, U_VolTexCalc.gVolTexCalcWork[9].y);
            temp_270 = fma(temp_264, temp_265, U_VolTexCalc.gVolTexCalcWork[9].z);
            temp_271 = fma(temp_267, temp_265, U_VolTexCalc.gVolTexCalcWork[9].w);
            temp_221 = false;
            temp_244 = floatBitsToInt(temp_268);
            temp_247 = floatBitsToInt(temp_269);
            temp_250 = floatBitsToInt(temp_270);
            temp_241 = floatBitsToInt(temp_271);
        }
        temp_272 = temp_221;
        temp_273 = temp_244;
        temp_274 = temp_247;
        temp_275 = temp_250;
        temp_276 = temp_241;
        temp_277 = temp_272;
        temp_278 = temp_276;
        temp_279 = temp_273;
        temp_280 = temp_274;
        temp_281 = temp_275;
        temp_245 = temp_273;
        temp_248 = temp_274;
        temp_251 = temp_275;
        temp_242 = temp_276;
        if (temp_272) {
            temp_239 = temp_252;
        }
        temp_282 = temp_239;
        temp_283 = temp_282;
        if (temp_272) {
            temp_284 = temp_282 + U_VolTexCalc.gVolTexCalcWork[3].w;
            temp_285 = temp_6 < temp_284;
            if (temp_285) {
                temp_277 = false;
            }
            temp_286 = temp_277;
            temp_287 = temp_286;
            if (temp_285) {
                temp_278 = floatBitsToInt(U_VolTexCalc.gVolTexCalcWork[8].w);
            }
            temp_288 = temp_278;
            temp_289 = temp_288;
            temp_242 = temp_288;
            if (temp_285) {
                temp_279 = floatBitsToInt(U_VolTexCalc.gVolTexCalcWork[8].x);
            }
            temp_290 = temp_279;
            temp_291 = temp_290;
            temp_245 = temp_290;
            if (temp_285) {
                temp_280 = floatBitsToInt(U_VolTexCalc.gVolTexCalcWork[8].y);
            }
            temp_292 = temp_280;
            temp_293 = temp_292;
            temp_248 = temp_292;
            if (temp_285) {
                temp_281 = floatBitsToInt(U_VolTexCalc.gVolTexCalcWork[8].z);
            }
            temp_294 = temp_281;
            temp_295 = temp_294;
            temp_251 = temp_294;
            if (temp_286) {
                temp_283 = temp_284;
            }
            temp_296 = temp_283;
            temp_297 = temp_296;
            if (temp_286) {
                temp_298 = temp_296 + U_VolTexCalc.gVolTexCalcWork[3].z;
                temp_299 = temp_6 < temp_298;
                if (temp_299) {
                    temp_300 = 0. - temp_296;
                    temp_301 = temp_300 + temp_298;
                    temp_302 = 1. / temp_301;
                    temp_303 = 0. - temp_296;
                    temp_304 = temp_303 + temp_6;
                    temp_305 = temp_304 * temp_302;
                    temp_306 = 0. - U_VolTexCalc.gVolTexCalcWork[8].x;
                    temp_307 = temp_306 + U_VolTexCalc.gVolTexCalcWork[7].x;
                    temp_308 = 0. - U_VolTexCalc.gVolTexCalcWork[8].z;
                    temp_309 = temp_308 + U_VolTexCalc.gVolTexCalcWork[7].z;
                    temp_310 = 0. - U_VolTexCalc.gVolTexCalcWork[8].w;
                    temp_311 = U_VolTexCalc.gVolTexCalcWork[7].w + temp_310;
                    temp_312 = 0. - U_VolTexCalc.gVolTexCalcWork[8].y;
                    temp_313 = temp_312 + U_VolTexCalc.gVolTexCalcWork[7].y;
                    temp_314 = fma(temp_307, temp_305, U_VolTexCalc.gVolTexCalcWork[8].x);
                    temp_315 = fma(temp_309, temp_305, U_VolTexCalc.gVolTexCalcWork[8].z);
                    temp_316 = fma(temp_313, temp_305, U_VolTexCalc.gVolTexCalcWork[8].y);
                    temp_317 = fma(temp_311, temp_305, U_VolTexCalc.gVolTexCalcWork[8].w);
                    temp_287 = false;
                    temp_291 = floatBitsToInt(temp_314);
                    temp_293 = floatBitsToInt(temp_316);
                    temp_295 = floatBitsToInt(temp_315);
                    temp_289 = floatBitsToInt(temp_317);
                }
                temp_318 = temp_287;
                temp_319 = temp_291;
                temp_320 = temp_293;
                temp_321 = temp_295;
                temp_322 = temp_289;
                temp_323 = temp_318;
                temp_324 = temp_322;
                temp_325 = temp_319;
                temp_326 = temp_320;
                temp_327 = temp_321;
                temp_245 = temp_319;
                temp_248 = temp_320;
                temp_251 = temp_321;
                temp_242 = temp_322;
                if (temp_318) {
                    temp_297 = temp_298;
                }
                temp_328 = temp_297;
                temp_329 = temp_328;
                if (temp_318) {
                    temp_330 = temp_328 + U_VolTexCalc.gVolTexCalcWork[3].y;
                    temp_331 = temp_6 < temp_330;
                    if (temp_331) {
                        temp_323 = false;
                    }
                    temp_332 = temp_323;
                    temp_333 = temp_332;
                    if (temp_331) {
                        temp_324 = floatBitsToInt(U_VolTexCalc.gVolTexCalcWork[7].w);
                    }
                    temp_334 = temp_324;
                    temp_335 = temp_334;
                    temp_242 = temp_334;
                    if (temp_331) {
                        temp_325 = floatBitsToInt(U_VolTexCalc.gVolTexCalcWork[7].x);
                    }
                    temp_336 = temp_325;
                    temp_337 = temp_336;
                    temp_245 = temp_336;
                    if (temp_331) {
                        temp_326 = floatBitsToInt(U_VolTexCalc.gVolTexCalcWork[7].y);
                    }
                    temp_338 = temp_326;
                    temp_339 = temp_338;
                    temp_248 = temp_338;
                    if (temp_331) {
                        temp_327 = floatBitsToInt(U_VolTexCalc.gVolTexCalcWork[7].z);
                    }
                    temp_340 = temp_327;
                    temp_341 = temp_340;
                    temp_251 = temp_340;
                    if (temp_332) {
                        temp_329 = temp_330;
                    }
                    temp_342 = temp_329;
                    temp_343 = temp_342;
                    if (temp_332) {
                        temp_344 = temp_342 + U_VolTexCalc.gVolTexCalcWork[3].x;
                        temp_345 = temp_6 < temp_344;
                        if (temp_345) {
                            temp_346 = 0. - temp_342;
                            temp_347 = temp_346 + temp_344;
                            temp_348 = 1. / temp_347;
                            temp_349 = 0. - temp_342;
                            temp_350 = temp_349 + temp_6;
                            temp_351 = temp_350 * temp_348;
                            temp_352 = 0. - U_VolTexCalc.gVolTexCalcWork[7].x;
                            temp_353 = temp_352 + U_VolTexCalc.gVolTexCalcWork[6].x;
                            temp_354 = 0. - U_VolTexCalc.gVolTexCalcWork[7].z;
                            temp_355 = temp_354 + U_VolTexCalc.gVolTexCalcWork[6].z;
                            temp_356 = 0. - U_VolTexCalc.gVolTexCalcWork[7].w;
                            temp_357 = U_VolTexCalc.gVolTexCalcWork[6].w + temp_356;
                            temp_358 = 0. - U_VolTexCalc.gVolTexCalcWork[7].y;
                            temp_359 = temp_358 + U_VolTexCalc.gVolTexCalcWork[6].y;
                            temp_360 = fma(temp_353, temp_351, U_VolTexCalc.gVolTexCalcWork[7].x);
                            temp_361 = fma(temp_355, temp_351, U_VolTexCalc.gVolTexCalcWork[7].z);
                            temp_362 = fma(temp_359, temp_351, U_VolTexCalc.gVolTexCalcWork[7].y);
                            temp_363 = fma(temp_357, temp_351, U_VolTexCalc.gVolTexCalcWork[7].w);
                            temp_333 = false;
                            temp_337 = floatBitsToInt(temp_360);
                            temp_339 = floatBitsToInt(temp_362);
                            temp_341 = floatBitsToInt(temp_361);
                            temp_335 = floatBitsToInt(temp_363);
                        }
                        temp_364 = temp_333;
                        temp_365 = temp_337;
                        temp_366 = temp_339;
                        temp_367 = temp_341;
                        temp_368 = temp_335;
                        temp_369 = temp_364;
                        temp_370 = temp_368;
                        temp_371 = temp_365;
                        temp_372 = temp_366;
                        temp_373 = temp_367;
                        temp_245 = temp_365;
                        temp_248 = temp_366;
                        temp_251 = temp_367;
                        temp_242 = temp_368;
                        if (temp_364) {
                            temp_343 = temp_344;
                        }
                        temp_374 = temp_343;
                        temp_375 = temp_374;
                        if (temp_364) {
                            temp_376 = temp_374 + U_VolTexCalc.gVolTexCalcWork[2].w;
                            temp_377 = temp_6 < temp_376;
                            if (temp_377) {
                                temp_369 = false;
                            }
                            temp_378 = temp_369;
                            temp_379 = temp_378;
                            if (temp_377) {
                                temp_370 = floatBitsToInt(U_VolTexCalc.gVolTexCalcWork[6].w);
                            }
                            temp_380 = temp_370;
                            temp_381 = temp_380;
                            temp_242 = temp_380;
                            if (temp_377) {
                                temp_371 = floatBitsToInt(U_VolTexCalc.gVolTexCalcWork[6].x);
                            }
                            temp_382 = temp_371;
                            temp_383 = temp_382;
                            temp_245 = temp_382;
                            if (temp_377) {
                                temp_372 = floatBitsToInt(U_VolTexCalc.gVolTexCalcWork[6].y);
                            }
                            temp_384 = temp_372;
                            temp_385 = temp_384;
                            temp_248 = temp_384;
                            if (temp_377) {
                                temp_373 = floatBitsToInt(U_VolTexCalc.gVolTexCalcWork[6].z);
                            }
                            temp_386 = temp_373;
                            temp_387 = temp_386;
                            temp_251 = temp_386;
                            if (temp_378) {
                                temp_375 = temp_376;
                            }
                            temp_388 = temp_375;
                            temp_389 = temp_388;
                            if (temp_378) {
                                temp_390 = temp_388 + U_VolTexCalc.gVolTexCalcWork[2].z;
                                temp_391 = temp_6 < temp_390;
                                if (temp_391) {
                                    temp_392 = 0. - temp_388;
                                    temp_393 = temp_392 + temp_390;
                                    temp_394 = 1. / temp_393;
                                    temp_395 = 0. - temp_388;
                                    temp_396 = temp_395 + temp_6;
                                    temp_397 = temp_396 * temp_394;
                                    temp_398 = 0. - U_VolTexCalc.gVolTexCalcWork[6].x;
                                    temp_399 = temp_398 + U_VolTexCalc.gVolTexCalcWork[5].x;
                                    temp_400 = 0. - U_VolTexCalc.gVolTexCalcWork[6].z;
                                    temp_401 = temp_400 + U_VolTexCalc.gVolTexCalcWork[5].z;
                                    temp_402 = 0. - U_VolTexCalc.gVolTexCalcWork[6].w;
                                    temp_403 = U_VolTexCalc.gVolTexCalcWork[5].w + temp_402;
                                    temp_404 = 0. - U_VolTexCalc.gVolTexCalcWork[6].y;
                                    temp_405 = temp_404 + U_VolTexCalc.gVolTexCalcWork[5].y;
                                    temp_406 = fma(temp_399, temp_397, U_VolTexCalc.gVolTexCalcWork[6].x);
                                    temp_407 = fma(temp_401, temp_397, U_VolTexCalc.gVolTexCalcWork[6].z);
                                    temp_408 = fma(temp_405, temp_397, U_VolTexCalc.gVolTexCalcWork[6].y);
                                    temp_409 = fma(temp_403, temp_397, U_VolTexCalc.gVolTexCalcWork[6].w);
                                    temp_379 = false;
                                    temp_383 = floatBitsToInt(temp_406);
                                    temp_385 = floatBitsToInt(temp_408);
                                    temp_387 = floatBitsToInt(temp_407);
                                    temp_381 = floatBitsToInt(temp_409);
                                }
                                temp_410 = temp_379;
                                temp_411 = temp_383;
                                temp_412 = temp_385;
                                temp_413 = temp_387;
                                temp_414 = temp_381;
                                temp_415 = temp_410;
                                temp_416 = temp_414;
                                temp_417 = temp_411;
                                temp_418 = temp_412;
                                temp_419 = temp_413;
                                temp_245 = temp_411;
                                temp_248 = temp_412;
                                temp_251 = temp_413;
                                temp_242 = temp_414;
                                if (temp_410) {
                                    temp_389 = temp_390;
                                }
                                temp_420 = temp_389;
                                temp_421 = temp_420;
                                if (temp_410) {
                                    temp_422 = temp_420 + U_VolTexCalc.gVolTexCalcWork[2].y;
                                    temp_423 = temp_6 < temp_422;
                                    if (temp_423) {
                                        temp_415 = false;
                                    }
                                    temp_424 = temp_415;
                                    if (temp_423) {
                                        temp_416 = floatBitsToInt(U_VolTexCalc.gVolTexCalcWork[5].w);
                                    }
                                    temp_425 = temp_416;
                                    temp_242 = temp_425;
                                    if (temp_423) {
                                        temp_417 = floatBitsToInt(U_VolTexCalc.gVolTexCalcWork[5].x);
                                    }
                                    temp_426 = temp_417;
                                    temp_245 = temp_426;
                                    if (temp_423) {
                                        temp_418 = floatBitsToInt(U_VolTexCalc.gVolTexCalcWork[5].y);
                                    }
                                    temp_427 = temp_418;
                                    temp_248 = temp_427;
                                    if (temp_423) {
                                        temp_419 = floatBitsToInt(U_VolTexCalc.gVolTexCalcWork[5].z);
                                    }
                                    temp_428 = temp_419;
                                    temp_251 = temp_428;
                                    if (temp_424) {
                                        temp_421 = temp_422;
                                    }
                                    temp_429 = temp_421;
                                    if (temp_424) {
                                        temp_430 = temp_429 + U_VolTexCalc.gVolTexCalcWork[2].x;
                                        temp_431 = temp_6 < temp_430;
                                        if (temp_431) {
                                            temp_432 = 0. - temp_429;
                                            temp_433 = temp_432 + temp_430;
                                            temp_434 = 1. / temp_433;
                                            temp_435 = 0. - temp_429;
                                            temp_436 = temp_435 + temp_6;
                                            temp_437 = 0. - U_VolTexCalc.gVolTexCalcWork[5].x;
                                            temp_438 = intBitsToFloat(temp_205) + temp_437;
                                            temp_439 = 0. - U_VolTexCalc.gVolTexCalcWork[5].y;
                                            temp_440 = intBitsToFloat(temp_216) + temp_439;
                                            temp_441 = 0. - U_VolTexCalc.gVolTexCalcWork[5].z;
                                            temp_442 = intBitsToFloat(temp_231) + temp_441;
                                            temp_443 = 0. - U_VolTexCalc.gVolTexCalcWork[5].w;
                                            temp_444 = intBitsToFloat(temp_227) + temp_443;
                                            temp_445 = temp_436 * temp_434;
                                            temp_446 = fma(temp_438, temp_445, U_VolTexCalc.gVolTexCalcWork[5].x);
                                            temp_447 = fma(temp_440, temp_445, U_VolTexCalc.gVolTexCalcWork[5].y);
                                            temp_448 = fma(temp_442, temp_445, U_VolTexCalc.gVolTexCalcWork[5].z);
                                            temp_449 = fma(temp_444, temp_445, U_VolTexCalc.gVolTexCalcWork[5].w);
                                            temp_245 = floatBitsToInt(temp_446);
                                            temp_248 = floatBitsToInt(temp_447);
                                            temp_251 = floatBitsToInt(temp_448);
                                            temp_242 = floatBitsToInt(temp_449);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    temp_450 = temp_245;
    temp_451 = temp_248;
    temp_452 = temp_251;
    temp_453 = temp_242;
    temp_454 = temp_167 * 0.5;
    temp_455 = in_attr5.z;
    temp_456 = temp_190 * 0.5;
    temp_457 = abs(temp_454);
    temp_458 = abs(temp_456);
    temp_459 = max(temp_457, temp_458);
    temp_460 = max(temp_459, 1.);
    temp_461 = 1. / temp_460;
    temp_462 = temp_456 * temp_461;
    temp_463 = temp_454 * temp_461;
    temp_464 = temp_455 * 8.;
    temp_465 = floor(temp_464);
    temp_466 = temp_462 >= 0.;
    temp_467 = abs(temp_462);
    temp_468 = inversesqrt(temp_467);
    temp_469 = temp_463 >= 0.;
    temp_470 = abs(temp_463);
    temp_471 = inversesqrt(temp_470);
    temp_472 = fma(temp_142, 0.5, 0.5);
    temp_473 = 1. / temp_468;
    temp_474 = !temp_466;
    temp_475 = temp_474 ? 0. : 1.;
    temp_476 = temp_465 * 0.003921569;
    temp_477 = fma(temp_145, 0.5, 0.5);
    temp_478 = fma(temp_157, 1000., 0.5);
    temp_479 = floor(temp_476);
    temp_480 = !temp_469;
    temp_481 = temp_480 ? 0. : 1.;
    temp_482 = temp_475 * 0.6666667;
    temp_483 = 1. / temp_471;
    temp_484 = 0. - temp_465;
    temp_485 = temp_464 + temp_484;
    temp_486 = fma(temp_481, 0.33333334, temp_482);
    temp_487 = 0. - temp_479;
    temp_488 = temp_476 + temp_487;
    temp_489 = temp_479 * 0.003921569;
    temp_490 = temp_486 + 0.01;
    out_attr0.x = intBitsToFloat(temp_450);
    out_attr0.y = intBitsToFloat(temp_451);
    out_attr0.z = intBitsToFloat(temp_452);
    out_attr0.w = intBitsToFloat(temp_453);
    out_attr1.x = temp_234;
    out_attr1.y = temp_35;
    out_attr1.z = 0.;
    out_attr1.w = 0.0043137255;
    out_attr2.x = temp_472;
    out_attr2.y = temp_477;
    out_attr2.z = temp_43;
    out_attr2.w = temp_478;
    out_attr3.x = temp_483;
    out_attr3.y = temp_473;
    out_attr3.z = 0.;
    out_attr3.w = temp_490;
    out_attr4.x = temp_485;
    out_attr4.y = temp_488;
    out_attr4.z = temp_489;
    out_attr4.w = 0.;
    return;
}
