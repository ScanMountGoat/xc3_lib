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
layout(binding = 6, std140) uniform _U_Mdl {
    vec4 gmWVP[4];
    vec4 gmWorld[3];
    vec4 gmWorldView[3];
    vec4 gMdlParm;
}U_Mdl;
layout(binding = 4, std140) uniform _U_Static {
    vec4 gmView[3];
    vec4 gmProj[4];
    vec4 gmViewProj[4];
    vec4 gmInvView[3];
    vec4 gBilMat[3];
    vec4 gBilYJiku[3];
    vec4 gEtcParm;
    vec4 gViewYVec;
    vec4 gCDep;
    vec4 gDitVal;
    vec4 gPreMat[4];
    vec4 gScreenSize;
    vec4 gJitter;
    vec4 gDitTMAAVal;
    vec4 gmProjNonJitter[4];
    vec4 gmDiffPreMat[4];
}U_Static;
layout(binding = 0, std430) buffer _U_Bone {
    uint data[];
}U_Bone;
layout(binding = 1, std430) buffer _U_OdB {
    uint data[];
}U_OdB;
layout(location = 0) in vec4 vPos;
layout(location = 1) in vec4 nWgtIdx;
layout(location = 2) in vec4 vTex0;
layout(location = 3) in vec4 vNormal;
layout(location = 4) in vec4 vTan;
layout(location = 0) out vec4 out_attr0;
layout(location = 1) out vec4 out_attr1;
layout(location = 2) out vec4 out_attr2;
layout(location = 3) out vec4 out_attr3;
layout(location = 4) out vec4 out_attr4;
layout(location = 5) out vec4 out_attr5;
layout(location = 6) out vec4 out_attr6;
layout(location = 7) out vec4 out_attr7;
void main() {
    precise float temp_0;
    int temp_1;
    int temp_2;
    precise float temp_3;
    uint temp_4;
    int temp_5;
    int temp_6;
    int temp_7;
    precise float temp_8;
    precise float temp_9;
    int temp_10;
    precise float temp_11;
    precise float temp_12;
    int temp_13;
    uint temp_14;
    precise float temp_15;
    int temp_16;
    uint temp_17;
    precise float temp_18;
    int temp_19;
    uint temp_20;
    precise float temp_21;
    int temp_22;
    uint temp_23;
    precise float temp_24;
    precise float temp_25;
    precise float temp_26;
    uint temp_27;
    precise float temp_28;
    int temp_29;
    uint temp_30;
    precise float temp_31;
    int temp_32;
    uint temp_33;
    precise float temp_34;
    int temp_35;
    uint temp_36;
    precise float temp_37;
    uint temp_38;
    precise float temp_39;
    int temp_40;
    uint temp_41;
    precise float temp_42;
    int temp_43;
    uint temp_44;
    precise float temp_45;
    int temp_46;
    uint temp_47;
    precise float temp_48;
    uint temp_49;
    precise float temp_50;
    int temp_51;
    uint temp_52;
    precise float temp_53;
    int temp_54;
    uint temp_55;
    precise float temp_56;
    int temp_57;
    uint temp_58;
    precise float temp_59;
    precise float temp_60;
    precise float temp_61;
    precise float temp_62;
    precise float temp_63;
    precise float temp_64;
    precise float temp_65;
    uint temp_66;
    precise float temp_67;
    int temp_68;
    uint temp_69;
    precise float temp_70;
    int temp_71;
    uint temp_72;
    precise float temp_73;
    int temp_74;
    uint temp_75;
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
    uint temp_88;
    precise float temp_89;
    int temp_90;
    uint temp_91;
    precise float temp_92;
    int temp_93;
    uint temp_94;
    precise float temp_95;
    int temp_96;
    uint temp_97;
    precise float temp_98;
    precise float temp_99;
    precise float temp_100;
    precise float temp_101;
    precise float temp_102;
    precise float temp_103;
    precise float temp_104;
    precise float temp_105;
    precise float temp_106;
    precise float temp_107;
    precise float temp_108;
    precise float temp_109;
    precise float temp_110;
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
    gl_PointSize = 1.;
    gl_Position.x = 0.;
    gl_Position.y = 0.;
    gl_Position.z = 0.;
    gl_Position.w = 1.;
    temp_0 = nWgtIdx.x;
    temp_1 = floatBitsToInt(temp_0) & 65535;
    temp_2 = temp_1 * 48;
    temp_3 = vNormal.x;
    temp_4 = floatBitsToUint(temp_0) >> 16;
    temp_5 = int(temp_4) * 48;
    temp_6 = temp_5 << 16;
    temp_7 = temp_6 + temp_2;
    temp_8 = vNormal.y;
    temp_9 = vPos.x;
    temp_10 = temp_7 + 16;
    temp_11 = vTan.x;
    temp_12 = vPos.y;
    temp_13 = temp_7 + 32;
    temp_14 = uint(temp_7) >> 2;
    temp_15 = uintBitsToFloat(U_Bone.data[int(temp_14)]);
    temp_16 = temp_7 + 4;
    temp_17 = uint(temp_16) >> 2;
    temp_18 = uintBitsToFloat(U_Bone.data[int(temp_17)]);
    temp_19 = temp_7 + 8;
    temp_20 = uint(temp_19) >> 2;
    temp_21 = uintBitsToFloat(U_Bone.data[int(temp_20)]);
    temp_22 = temp_7 + 12;
    temp_23 = uint(temp_22) >> 2;
    temp_24 = uintBitsToFloat(U_Bone.data[int(temp_23)]);
    temp_25 = vTan.y;
    temp_26 = vNormal.z;
    temp_27 = uint(temp_10) >> 2;
    temp_28 = uintBitsToFloat(U_Bone.data[int(temp_27)]);
    temp_29 = temp_10 + 4;
    temp_30 = uint(temp_29) >> 2;
    temp_31 = uintBitsToFloat(U_Bone.data[int(temp_30)]);
    temp_32 = temp_10 + 8;
    temp_33 = uint(temp_32) >> 2;
    temp_34 = uintBitsToFloat(U_Bone.data[int(temp_33)]);
    temp_35 = temp_10 + 12;
    temp_36 = uint(temp_35) >> 2;
    temp_37 = uintBitsToFloat(U_Bone.data[int(temp_36)]);
    temp_38 = uint(temp_13) >> 2;
    temp_39 = uintBitsToFloat(U_Bone.data[int(temp_38)]);
    temp_40 = temp_13 + 4;
    temp_41 = uint(temp_40) >> 2;
    temp_42 = uintBitsToFloat(U_Bone.data[int(temp_41)]);
    temp_43 = temp_13 + 8;
    temp_44 = uint(temp_43) >> 2;
    temp_45 = uintBitsToFloat(U_Bone.data[int(temp_44)]);
    temp_46 = temp_13 + 12;
    temp_47 = uint(temp_46) >> 2;
    temp_48 = uintBitsToFloat(U_Bone.data[int(temp_47)]);
    temp_49 = uint(temp_7) >> 2;
    temp_50 = uintBitsToFloat(U_OdB.data[int(temp_49)]);
    temp_51 = temp_7 + 4;
    temp_52 = uint(temp_51) >> 2;
    temp_53 = uintBitsToFloat(U_OdB.data[int(temp_52)]);
    temp_54 = temp_7 + 8;
    temp_55 = uint(temp_54) >> 2;
    temp_56 = uintBitsToFloat(U_OdB.data[int(temp_55)]);
    temp_57 = temp_7 + 12;
    temp_58 = uint(temp_57) >> 2;
    temp_59 = uintBitsToFloat(U_OdB.data[int(temp_58)]);
    temp_60 = temp_15 * temp_3;
    temp_61 = fma(temp_18, temp_8, temp_60);
    temp_62 = temp_39 * temp_3;
    temp_63 = temp_28 * temp_3;
    temp_64 = fma(temp_42, temp_8, temp_62);
    temp_65 = fma(temp_31, temp_8, temp_63);
    temp_66 = uint(temp_10) >> 2;
    temp_67 = uintBitsToFloat(U_OdB.data[int(temp_66)]);
    temp_68 = temp_10 + 4;
    temp_69 = uint(temp_68) >> 2;
    temp_70 = uintBitsToFloat(U_OdB.data[int(temp_69)]);
    temp_71 = temp_10 + 8;
    temp_72 = uint(temp_71) >> 2;
    temp_73 = uintBitsToFloat(U_OdB.data[int(temp_72)]);
    temp_74 = temp_10 + 12;
    temp_75 = uint(temp_74) >> 2;
    temp_76 = uintBitsToFloat(U_OdB.data[int(temp_75)]);
    temp_77 = temp_15 * temp_11;
    temp_78 = temp_50 * temp_9;
    temp_79 = temp_15 * temp_9;
    temp_80 = temp_28 * temp_9;
    temp_81 = temp_28 * temp_11;
    temp_82 = fma(temp_18, temp_12, temp_79);
    temp_83 = fma(temp_18, temp_25, temp_77);
    temp_84 = temp_39 * temp_11;
    temp_85 = fma(temp_31, temp_12, temp_80);
    temp_86 = fma(temp_31, temp_25, temp_81);
    temp_87 = fma(temp_42, temp_25, temp_84);
    temp_88 = uint(temp_13) >> 2;
    temp_89 = uintBitsToFloat(U_OdB.data[int(temp_88)]);
    temp_90 = temp_13 + 4;
    temp_91 = uint(temp_90) >> 2;
    temp_92 = uintBitsToFloat(U_OdB.data[int(temp_91)]);
    temp_93 = temp_13 + 8;
    temp_94 = uint(temp_93) >> 2;
    temp_95 = uintBitsToFloat(U_OdB.data[int(temp_94)]);
    temp_96 = temp_13 + 12;
    temp_97 = uint(temp_96) >> 2;
    temp_98 = uintBitsToFloat(U_OdB.data[int(temp_97)]);
    temp_99 = fma(temp_53, temp_12, temp_78);
    temp_100 = vPos.z;
    temp_101 = fma(temp_21, temp_26, temp_61);
    temp_102 = temp_39 * temp_9;
    temp_103 = fma(temp_45, temp_26, temp_64);
    temp_104 = fma(temp_21, temp_100, temp_82);
    temp_105 = fma(temp_56, temp_100, temp_99);
    temp_106 = vTex0.x;
    temp_107 = fma(temp_34, temp_26, temp_65);
    temp_108 = fma(temp_34, temp_100, temp_85);
    temp_109 = fma(temp_42, temp_12, temp_102);
    temp_110 = fma(temp_45, temp_100, temp_109);
    temp_111 = temp_100 * U_Mdl.gMdlParm.y;
    temp_112 = temp_67 * temp_9;
    temp_113 = fma(temp_70, temp_12, temp_112);
    temp_114 = vTan.z;
    temp_115 = fma(temp_21, temp_114, temp_83);
    temp_116 = vPos.w;
    temp_117 = fma(temp_34, temp_114, temp_86);
    temp_118 = fma(temp_45, temp_114, temp_87);
    temp_119 = fma(temp_73, temp_100, temp_113);
    temp_120 = temp_89 * temp_9;
    temp_121 = vTan.w;
    temp_122 = temp_103 * temp_117;
    temp_123 = temp_118 * temp_101;
    temp_124 = fma(temp_92, temp_12, temp_120);
    temp_125 = vTex0.y;
    temp_126 = 0. - temp_123;
    temp_127 = fma(temp_103, temp_115, temp_126);
    out_attr4.x = temp_106;
    temp_128 = fma(temp_59, temp_116, temp_105);
    out_attr1.x = temp_115;
    temp_129 = fma(temp_76, temp_116, temp_119);
    out_attr7.z = temp_111;
    temp_130 = 0. - temp_122;
    temp_131 = fma(temp_118, temp_107, temp_130);
    out_attr0.x = temp_101;
    temp_132 = temp_128 * U_Static.gmProj[2].x;
    out_attr4.y = temp_125;
    temp_133 = fma(temp_24, temp_116, temp_104);
    out_attr0.y = temp_107;
    temp_134 = temp_107 * temp_115;
    out_attr1.z = temp_118;
    temp_135 = fma(temp_95, temp_100, temp_124);
    out_attr0.z = temp_103;
    temp_136 = fma(temp_129, U_Static.gmProj[2].y, temp_132);
    out_attr1.y = temp_117;
    temp_137 = fma(temp_37, temp_116, temp_108);
    out_attr3.x = temp_133;
    temp_138 = temp_133 * U_Static.gmProj[3].x;
    out_attr3.y = temp_137;
    temp_139 = fma(temp_98, temp_116, temp_135);
    temp_140 = fma(temp_48, temp_116, temp_110);
    temp_141 = fma(temp_137, U_Static.gmProj[3].y, temp_138);
    out_attr3.z = temp_140;
    temp_142 = temp_128 * U_Static.gmProj[3].x;
    temp_143 = temp_131 * temp_121;
    temp_144 = temp_128 * U_Static.gmProj[1].x;
    out_attr2.x = temp_143;
    temp_145 = fma(temp_140, U_Static.gmProj[3].z, temp_141);
    temp_146 = temp_128 * U_Static.gmProj[0].x;
    temp_147 = temp_127 * temp_121;
    temp_148 = temp_145 + U_Static.gmProj[3].w;
    out_attr2.y = temp_147;
    temp_149 = temp_133 * U_Static.gmProj[2].x;
    gl_Position.w = temp_148;
    temp_150 = fma(temp_129, U_Static.gmProj[0].y, temp_146);
    out_attr5.w = temp_148;
    temp_151 = temp_133 * U_Static.gmProj[0].x;
    temp_152 = fma(temp_137, U_Static.gmProj[2].y, temp_149);
    temp_153 = fma(temp_129, U_Static.gmProj[3].y, temp_142);
    temp_154 = fma(temp_139, U_Static.gmProj[0].z, temp_150);
    temp_155 = fma(temp_129, U_Static.gmProj[1].y, temp_144);
    temp_156 = fma(temp_137, U_Static.gmProj[0].y, temp_151);
    temp_157 = temp_133 * U_Static.gmProj[1].x;
    temp_158 = fma(temp_140, U_Static.gmProj[2].z, temp_152);
    temp_159 = temp_154 + U_Static.gmProj[0].w;
    out_attr6.x = temp_159;
    temp_160 = temp_158 + U_Static.gmProj[2].w;
    temp_161 = fma(temp_137, U_Static.gmProj[1].y, temp_157);
    gl_Position.z = temp_160;
    temp_162 = 0. - temp_134;
    temp_163 = fma(temp_117, temp_101, temp_162);
    temp_164 = fma(temp_139, U_Static.gmProj[3].z, temp_153);
    temp_165 = fma(temp_139, U_Static.gmProj[2].z, temp_136);
    temp_166 = fma(temp_139, U_Static.gmProj[1].z, temp_155);
    temp_167 = fma(temp_140, U_Static.gmProj[0].z, temp_156);
    temp_168 = fma(temp_140, U_Static.gmProj[1].z, temp_161);
    temp_169 = 0. - U_Static.gCDep.y;
    temp_170 = temp_160 + temp_169;
    temp_171 = temp_9 * U_Mdl.gMdlParm.y;
    temp_172 = temp_12 * U_Mdl.gMdlParm.y;
    out_attr7.x = temp_171;
    temp_173 = temp_163 * temp_121;
    out_attr7.y = temp_172;
    temp_174 = temp_164 + U_Static.gmProj[3].w;
    out_attr2.z = temp_173;
    temp_175 = temp_165 + U_Static.gmProj[2].w;
    out_attr6.w = temp_174;
    temp_176 = temp_166 + U_Static.gmProj[1].w;
    out_attr6.z = temp_175;
    temp_177 = temp_167 + U_Static.gmProj[0].w;
    out_attr6.y = temp_176;
    temp_178 = temp_168 + U_Static.gmProj[1].w;
    gl_Position.x = temp_177;
    temp_179 = temp_170 * U_Static.gCDep.z;
    out_attr5.x = temp_177;
    gl_Position.y = temp_178;
    out_attr5.y = temp_178;
    out_attr5.z = temp_179;
    return;
}
