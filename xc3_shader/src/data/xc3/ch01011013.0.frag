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
layout(binding = 5, std140) uniform _U_Mate {
    vec4 gWrkFl4[2];
    vec4 gWrkCol;
    vec4 gMatCol;
}U_Mate;
layout(binding = 2, std140) uniform _fp_c1 {
    precise vec4 data[4096];
}fp_c1;
layout(location = 0) in vec4 in_attr0;
layout(location = 1) in vec4 in_attr1;
layout(location = 2) in vec4 in_attr2;
layout(location = 3) in vec4 in_attr3;
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
    bool temp_4;
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
    precise float temp_30;
    precise float temp_31;
    precise float temp_32;
    precise float temp_33;
    precise float temp_34;
    precise float temp_35;
    precise float temp_36;
    precise float temp_37;
    precise float temp_38;
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
    precise float temp_54;
    precise float temp_55;
    precise float temp_56;
    precise float temp_57;
    precise float temp_58;
    precise float temp_59;
    precise float temp_60;
    bool temp_61;
    precise float temp_62;
    precise float temp_63;
    precise float temp_64;
    precise float temp_65;
    bool temp_66;
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
    temp_0 = in_attr1.w;
    temp_1 = in_attr0.x;
    temp_2 = in_attr0.y;
    temp_3 = in_attr0.z;
    temp_4 = temp_0 <= 0.;
    temp_5 = in_attr3.w;
    if (temp_4) {
        discard;
    }
    temp_6 = in_attr2.w;
    temp_7 = temp_1 * temp_1;
    temp_8 = in_attr3.x;
    temp_9 = fma(temp_2, temp_2, temp_7);
    temp_10 = in_attr3.y;
    temp_11 = fma(temp_3, temp_3, temp_9);
    temp_12 = in_attr2.x;
    temp_13 = in_attr2.y;
    temp_14 = inversesqrt(temp_11);
    temp_15 = 1. / temp_5;
    temp_16 = in_attr2.z;
    temp_17 = in_attr1.z;
    temp_18 = temp_1 * temp_14;
    temp_19 = 1. / temp_6;
    temp_20 = temp_2 * temp_14;
    temp_21 = temp_8 * temp_15;
    temp_22 = temp_3 * temp_14;
    temp_23 = temp_10 * temp_15;
    temp_24 = 0. - temp_21;
    temp_25 = fma(temp_19, temp_12, temp_24);
    temp_26 = in_attr1.x;
    temp_27 = temp_16 * 8.;
    temp_28 = 0. - temp_23;
    temp_29 = fma(temp_19, temp_13, temp_28);
    temp_30 = in_attr1.y;
    temp_31 = temp_25 * 0.5;
    temp_32 = temp_29 * 0.5;
    temp_33 = dFdy(temp_18);
    temp_34 = dFdy(temp_20);
    temp_35 = abs(temp_31);
    temp_36 = abs(temp_32);
    temp_37 = max(temp_35, temp_36);
    temp_38 = 1. * temp_33;
    temp_39 = dFdx(temp_18);
    temp_40 = 1. * temp_34;
    temp_41 = max(temp_37, 1.);
    temp_42 = temp_38 * temp_38;
    temp_43 = 1. / temp_41;
    temp_44 = dFdy(temp_22);
    temp_45 = temp_39 * temp_39;
    temp_46 = dFdx(temp_20);
    temp_47 = temp_26 * U_Mate.gMatCol.x;
    temp_48 = fma(temp_40, temp_40, temp_42);
    temp_49 = 1. * temp_44;
    temp_50 = floor(temp_27);
    temp_51 = dFdx(temp_22);
    temp_52 = fma(temp_46, temp_46, temp_45);
    temp_53 = temp_30 * U_Mate.gMatCol.y;
    temp_54 = temp_47 * 0.01;
    temp_55 = temp_17 * U_Mate.gMatCol.z;
    temp_56 = temp_32 * temp_43;
    temp_57 = fma(temp_49, temp_49, temp_48);
    temp_58 = fma(temp_51, temp_51, temp_52);
    temp_59 = temp_31 * temp_43;
    temp_60 = fma(temp_53, 0.01, temp_54);
    temp_61 = temp_56 >= 0.;
    temp_62 = temp_61 ? 1. : 0.;
    temp_63 = abs(temp_56);
    temp_64 = inversesqrt(temp_63);
    temp_65 = temp_58 + temp_57;
    temp_66 = temp_59 >= 0.;
    temp_67 = temp_66 ? 1. : 0.;
    temp_68 = abs(temp_59);
    temp_69 = inversesqrt(temp_68);
    temp_70 = fma(temp_55, 0.01, temp_60);
    temp_71 = 0. - temp_50;
    temp_72 = temp_27 + temp_71;
    temp_73 = temp_50 * 0.003921569;
    temp_74 = 1. / temp_64;
    temp_75 = temp_62 * 0.6666667;
    temp_76 = 1. / temp_69;
    temp_77 = temp_65 * 0.5;
    temp_78 = 0. - temp_47;
    temp_79 = temp_78 + temp_70;
    temp_80 = 0. - temp_53;
    temp_81 = temp_80 + temp_70;
    temp_82 = fma(temp_67, 0.33333334, temp_75);
    temp_83 = min(temp_77, 0.18);
    temp_84 = floor(temp_73);
    temp_85 = fma(temp_79, U_Mate.gWrkFl4[1].z, temp_47);
    temp_86 = 0. - temp_55;
    temp_87 = temp_86 + temp_70;
    temp_88 = fma(temp_81, U_Mate.gWrkFl4[1].z, temp_53);
    temp_89 = temp_83 + 1.;
    temp_90 = clamp(temp_89, 0., 1.);
    temp_91 = sqrt(temp_90);
    temp_92 = fma(temp_87, U_Mate.gWrkFl4[1].z, temp_55);
    temp_93 = fma(temp_18, 0.5, 0.5);
    temp_94 = fma(temp_20, 0.5, 0.5);
    temp_95 = fma(temp_22, 1000., 0.5);
    temp_96 = 0. - temp_84;
    temp_97 = temp_73 + temp_96;
    temp_98 = temp_84 * 0.003921569;
    temp_99 = temp_82 + 0.01;
    temp_100 = 0. - temp_91;
    temp_101 = temp_100 + 1.;
    out_attr0.x = temp_85;
    out_attr0.y = temp_88;
    out_attr0.z = temp_92;
    out_attr0.w = 0.;
    out_attr1.x = 0.;
    out_attr1.y = temp_101;
    out_attr1.z = U_Mate.gWrkFl4[0].x;
    out_attr1.w = 0.039607845;
    out_attr2.x = temp_93;
    out_attr2.y = temp_94;
    out_attr2.z = 1.;
    out_attr2.w = temp_95;
    out_attr3.x = temp_76;
    out_attr3.y = temp_74;
    out_attr3.z = 0.;
    out_attr3.w = temp_99;
    out_attr4.x = temp_72;
    out_attr4.y = temp_97;
    out_attr4.z = temp_98;
    out_attr4.w = 0.5;
    return;
}