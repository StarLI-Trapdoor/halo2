#![allow(clippy::many_single_char_names)]
#![allow(clippy::op_ref)]

use group::Curve;
use halo2::arithmetic::FieldExt;
use halo2::circuit::{Cell, Layouter, SimpleFloorPlanner};
use halo2::dev::MockProver;
use halo2::pasta::{EqAffine, Fp};
use halo2::plonk::{
    create_proof, keygen_pk, keygen_vk, verify_proof, Advice, Circuit, Column, ConstraintSystem,
    Error, Fixed, Permutation, VerifyingKey,
};
use halo2::poly::{
    commitment::{Blind, Params},
    Rotation,
};
use halo2::transcript::{Blake2bRead, Blake2bWrite, Challenge255};
use std::marker::PhantomData;

#[test]
fn plonk_api() {
    const K: u32 = 5;

    /// This represents an advice column at a certain row in the ConstraintSystem
    #[derive(Copy, Clone, Debug)]
    pub struct Variable(Column<Advice>, usize);

    // Initialize the polynomial commitment parameters
    let params: Params<EqAffine> = Params::new(K);

    #[derive(Clone)]
    struct PlonkConfig {
        a: Column<Advice>,
        b: Column<Advice>,
        c: Column<Advice>,
        d: Column<Advice>,
        e: Column<Advice>,

        sa: Column<Fixed>,
        sb: Column<Fixed>,
        sc: Column<Fixed>,
        sm: Column<Fixed>,
        sp: Column<Fixed>,
        sl: Column<Fixed>,
        sl2: Column<Fixed>,

        perm: Permutation,
        perm2: Permutation,
    }

    trait StandardCs<FF: FieldExt> {
        fn raw_multiply<F>(
            &self,
            layouter: &mut impl Layouter<FF>,
            f: F,
        ) -> Result<(Cell, Cell, Cell), Error>
        where
            F: FnMut() -> Result<(FF, FF, FF), Error>;
        fn raw_add<F>(
            &self,
            layouter: &mut impl Layouter<FF>,
            f: F,
        ) -> Result<(Cell, Cell, Cell), Error>
        where
            F: FnMut() -> Result<(FF, FF, FF), Error>;
        fn copy(&self, layouter: &mut impl Layouter<FF>, a: Cell, b: Cell) -> Result<(), Error>;
        fn public_input<F>(&self, layouter: &mut impl Layouter<FF>, f: F) -> Result<Cell, Error>
        where
            F: FnMut() -> Result<FF, Error>;
        fn lookup_table(
            &self,
            layouter: &mut impl Layouter<FF>,
            values: &[Vec<FF>],
        ) -> Result<(), Error>;
    }

    #[derive(Clone)]
    struct MyCircuit<F: FieldExt> {
        a: Option<F>,
        lookup_tables: Vec<Vec<F>>,
    }

    struct StandardPlonk<F: FieldExt> {
        config: PlonkConfig,
        _marker: PhantomData<F>,
    }

    impl<FF: FieldExt> StandardPlonk<FF> {
        fn new(config: PlonkConfig) -> Self {
            StandardPlonk {
                config,
                _marker: PhantomData,
            }
        }
    }

    impl<FF: FieldExt> StandardCs<FF> for StandardPlonk<FF> {
        fn raw_multiply<F>(
            &self,
            layouter: &mut impl Layouter<FF>,
            mut f: F,
        ) -> Result<(Cell, Cell, Cell), Error>
        where
            F: FnMut() -> Result<(FF, FF, FF), Error>,
        {
            layouter.assign_region(
                || "raw_multiply",
                |mut region| {
                    let mut value = None;
                    let lhs = region.assign_advice(
                        || "lhs",
                        self.config.a,
                        0,
                        || {
                            value = Some(f()?);
                            Ok(value.ok_or(Error::SynthesisError)?.0)
                        },
                    )?;
                    region.assign_advice(
                        || "lhs^4",
                        self.config.d,
                        0,
                        || Ok(value.ok_or(Error::SynthesisError)?.0.square().square()),
                    )?;
                    let rhs = region.assign_advice(
                        || "rhs",
                        self.config.b,
                        0,
                        || Ok(value.ok_or(Error::SynthesisError)?.1),
                    )?;
                    region.assign_advice(
                        || "rhs^4",
                        self.config.e,
                        0,
                        || Ok(value.ok_or(Error::SynthesisError)?.1.square().square()),
                    )?;
                    let out = region.assign_advice(
                        || "out",
                        self.config.c,
                        0,
                        || Ok(value.ok_or(Error::SynthesisError)?.2),
                    )?;

                    region.assign_fixed(|| "a", self.config.sa, 0, || Ok(FF::zero()))?;
                    region.assign_fixed(|| "b", self.config.sb, 0, || Ok(FF::zero()))?;
                    region.assign_fixed(|| "c", self.config.sc, 0, || Ok(FF::one()))?;
                    region.assign_fixed(|| "a * b", self.config.sm, 0, || Ok(FF::one()))?;
                    Ok((lhs, rhs, out))
                },
            )
        }
        fn raw_add<F>(
            &self,
            layouter: &mut impl Layouter<FF>,
            mut f: F,
        ) -> Result<(Cell, Cell, Cell), Error>
        where
            F: FnMut() -> Result<(FF, FF, FF), Error>,
        {
            layouter.assign_region(
                || "raw_add",
                |mut region| {
                    let mut value = None;
                    let lhs = region.assign_advice(
                        || "lhs",
                        self.config.a,
                        0,
                        || {
                            value = Some(f()?);
                            Ok(value.ok_or(Error::SynthesisError)?.0)
                        },
                    )?;
                    region.assign_advice(
                        || "lhs^4",
                        self.config.d,
                        0,
                        || Ok(value.ok_or(Error::SynthesisError)?.0.square().square()),
                    )?;
                    let rhs = region.assign_advice(
                        || "rhs",
                        self.config.b,
                        0,
                        || Ok(value.ok_or(Error::SynthesisError)?.1),
                    )?;
                    region.assign_advice(
                        || "rhs^4",
                        self.config.e,
                        0,
                        || Ok(value.ok_or(Error::SynthesisError)?.1.square().square()),
                    )?;
                    let out = region.assign_advice(
                        || "out",
                        self.config.c,
                        0,
                        || Ok(value.ok_or(Error::SynthesisError)?.2),
                    )?;

                    region.assign_fixed(|| "a", self.config.sa, 0, || Ok(FF::one()))?;
                    region.assign_fixed(|| "b", self.config.sb, 0, || Ok(FF::one()))?;
                    region.assign_fixed(|| "c", self.config.sc, 0, || Ok(FF::one()))?;
                    region.assign_fixed(|| "a * b", self.config.sm, 0, || Ok(FF::zero()))?;
                    Ok((lhs, rhs, out))
                },
            )
        }
        fn copy(
            &self,
            layouter: &mut impl Layouter<FF>,
            left: Cell,
            right: Cell,
        ) -> Result<(), Error> {
            layouter.assign_region(
                || "copy",
                |mut region| {
                    region.constrain_equal(&self.config.perm, left, right)?;
                    region.constrain_equal(&self.config.perm2, left, right)
                },
            )
        }
        fn public_input<F>(&self, layouter: &mut impl Layouter<FF>, mut f: F) -> Result<Cell, Error>
        where
            F: FnMut() -> Result<FF, Error>,
        {
            layouter.assign_region(
                || "public_input",
                |mut region| {
                    let value = region.assign_advice(|| "value", self.config.a, 0, || f())?;
                    region.assign_fixed(|| "public", self.config.sp, 0, || Ok(FF::one()))?;

                    Ok(value)
                },
            )
        }
        fn lookup_table(
            &self,
            layouter: &mut impl Layouter<FF>,
            values: &[Vec<FF>],
        ) -> Result<(), Error> {
            layouter.assign_region(
                || "",
                |mut region| {
                    for (index, (&value_0, &value_1)) in
                        values[0].iter().zip(values[1].iter()).enumerate()
                    {
                        region.assign_fixed(
                            || "table col 1",
                            self.config.sl,
                            index,
                            || Ok(value_0),
                        )?;
                        region.assign_fixed(
                            || "table col 2",
                            self.config.sl2,
                            index,
                            || Ok(value_1),
                        )?;
                    }
                    Ok(())
                },
            )?;
            Ok(())
        }
    }

    impl<F: FieldExt> Circuit<F> for MyCircuit<F> {
        type Config = PlonkConfig;
        type FloorPlanner = SimpleFloorPlanner;

        fn without_witnesses(&self) -> Self {
            Self {
                a: None,
                lookup_tables: self.lookup_tables.clone(),
            }
        }

        fn configure(meta: &mut ConstraintSystem<F>) -> PlonkConfig {
            let e = meta.advice_column();
            let a = meta.advice_column();
            let b = meta.advice_column();
            let sf = meta.fixed_column();
            let c = meta.advice_column();
            let d = meta.advice_column();
            let p = meta.instance_column();

            let perm = meta.permutation(&[a.into(), b.into(), c.into()]);
            let perm2 = meta.permutation(&[a.into(), b.into(), c.into()]);

            let sm = meta.fixed_column();
            let sa = meta.fixed_column();
            let sb = meta.fixed_column();
            let sc = meta.fixed_column();
            let sp = meta.fixed_column();
            let sl = meta.fixed_column();
            let sl2 = meta.fixed_column();

            /*
             *   A         B      ...  sl        sl2
             * [
             *   instance  0      ...  0         0
             *   a         a      ...  0         0
             *   a         a^2    ...  0         0
             *   a         a      ...  0         0
             *   a         a^2    ...  0         0
             *   ...       ...    ...  ...       ...
             *   ...       ...    ...  instance  0
             *   ...       ...    ...  a         a
             *   ...       ...    ...  a         a^2
             *   ...       ...    ...  0         0
             * ]
             */
            meta.lookup(|meta| {
                let a_ = meta.query_any(a.into(), Rotation::cur());
                let sl_ = meta.query_any(sl.into(), Rotation::cur());
                vec![(a_, sl_)]
            });
            meta.lookup(|meta| {
                let a_ = meta.query_any(a.into(), Rotation::cur());
                let b_ = meta.query_any(b.into(), Rotation::cur());
                let sl_ = meta.query_any(sl.into(), Rotation::cur());
                let sl2_ = meta.query_any(sl2.into(), Rotation::cur());
                vec![(a_ * b_, sl_ * sl2_)]
            });

            meta.create_gate("Combined add-mult", |meta| {
                let d = meta.query_advice(d, Rotation::next());
                let a = meta.query_advice(a, Rotation::cur());
                let sf = meta.query_fixed(sf, Rotation::cur());
                let e = meta.query_advice(e, Rotation::prev());
                let b = meta.query_advice(b, Rotation::cur());
                let c = meta.query_advice(c, Rotation::cur());

                let sa = meta.query_fixed(sa, Rotation::cur());
                let sb = meta.query_fixed(sb, Rotation::cur());
                let sc = meta.query_fixed(sc, Rotation::cur());
                let sm = meta.query_fixed(sm, Rotation::cur());

                vec![
                    a.clone() * sa
                        + b.clone() * sb
                        + a * b * sm
                        + (c * sc * (-F::one()))
                        + sf * (d * e),
                ]
            });

            meta.create_gate("Public input", |meta| {
                let a = meta.query_advice(a, Rotation::cur());
                let p = meta.query_instance(p, Rotation::cur());
                let sp = meta.query_fixed(sp, Rotation::cur());

                vec![sp * (a + p * (-F::one()))]
            });

            PlonkConfig {
                a,
                b,
                c,
                d,
                e,
                sa,
                sb,
                sc,
                sm,
                sp,
                sl,
                sl2,
                perm,
                perm2,
            }
        }

        fn synthesize(
            &self,
            config: PlonkConfig,
            mut layouter: impl Layouter<F>,
        ) -> Result<(), Error> {
            let cs = StandardPlonk::new(config);

            let _ = cs.public_input(&mut layouter, || Ok(F::one() + F::one()))?;

            for _ in 0..10 {
                let mut a_squared = None;
                let (a0, _, c0) = cs.raw_multiply(&mut layouter, || {
                    a_squared = self.a.map(|a| a.square());
                    Ok((
                        self.a.ok_or(Error::SynthesisError)?,
                        self.a.ok_or(Error::SynthesisError)?,
                        a_squared.ok_or(Error::SynthesisError)?,
                    ))
                })?;
                let (a1, b1, _) = cs.raw_add(&mut layouter, || {
                    let fin = a_squared.and_then(|a2| self.a.map(|a| a + a2));
                    Ok((
                        self.a.ok_or(Error::SynthesisError)?,
                        a_squared.ok_or(Error::SynthesisError)?,
                        fin.ok_or(Error::SynthesisError)?,
                    ))
                })?;
                cs.copy(&mut layouter, a0, a1)?;
                cs.copy(&mut layouter, b1, c0)?;
            }

            cs.lookup_table(&mut layouter, &self.lookup_tables)?;

            Ok(())
        }
    }

    let a = Fp::from_u64(2834758237) * Fp::ZETA;
    let a_squared = a * &a;
    let instance = Fp::one() + Fp::one();
    let lookup_table = vec![instance, a, a, Fp::zero()];
    let lookup_table_2 = vec![Fp::zero(), a, a_squared, Fp::zero()];

    let empty_circuit: MyCircuit<Fp> = MyCircuit {
        a: None,
        lookup_tables: vec![lookup_table.clone(), lookup_table_2.clone()],
    };

    let circuit: MyCircuit<Fp> = MyCircuit {
        a: Some(a),
        lookup_tables: vec![lookup_table, lookup_table_2],
    };

    // Initialize the proving key
    let vk = keygen_vk(&params, &empty_circuit).expect("keygen_vk should not fail");
    let pk = keygen_pk(&params, vk, &empty_circuit).expect("keygen_pk should not fail");

    let mut pubinputs = pk.get_vk().get_domain().empty_lagrange();
    pubinputs[0] = instance;
    let pubinput = params
        .commit_lagrange(&pubinputs, Blind::default())
        .to_affine();

    // Check this circuit is satisfied.
    let prover = match MockProver::run(K, &circuit, vec![pubinputs.to_vec()]) {
        Ok(prover) => prover,
        Err(e) => panic!("{:?}", e),
    };
    assert_eq!(prover.verify(), Ok(()));

    for _ in 0..10 {
        let mut transcript = Blake2bWrite::<_, _, Challenge255<_>>::init(vec![]);
        // Create a proof
        create_proof(
            &params,
            &pk,
            &[circuit.clone(), circuit.clone()],
            &[&[pubinputs.clone()], &[pubinputs.clone()]],
            &mut transcript,
        )
        .expect("proof generation should not fail");
        let proof: Vec<u8> = transcript.finalize();

        let pubinput_slice = &[pubinput];
        let pubinput_slice_copy = &[pubinput];
        let msm = params.empty_msm();
        let mut transcript = Blake2bRead::<_, _, Challenge255<_>>::init(&proof[..]);
        let guard = verify_proof(
            &params,
            pk.get_vk(),
            msm,
            &[pubinput_slice, pubinput_slice_copy],
            &mut transcript,
        )
        .unwrap();
        {
            let msm = guard.clone().use_challenges();
            assert!(msm.eval());
        }
        {
            let g = guard.compute_g();
            let (msm, _) = guard.clone().use_g(g);
            assert!(msm.eval());
        }
        let msm = guard.clone().use_challenges();
        assert!(msm.clone().eval());
        let mut transcript = Blake2bRead::<_, _, Challenge255<_>>::init(&proof[..]);
        let mut vk_buffer = vec![];
        pk.get_vk().write(&mut vk_buffer).unwrap();
        let vk = VerifyingKey::<EqAffine>::read::<_, MyCircuit<Fp>>(&mut &vk_buffer[..], &params)
            .unwrap();
        let guard = verify_proof(
            &params,
            &vk,
            msm,
            &[pubinput_slice, pubinput_slice_copy],
            &mut transcript,
        )
        .unwrap();
        {
            let msm = guard.clone().use_challenges();
            assert!(msm.eval());
        }
        {
            let g = guard.compute_g();
            let (msm, _) = guard.clone().use_g(g);
            assert!(msm.eval());
        }
    }

    // Check that the verification key has not changed unexpectedly
    {
        assert_eq!(
            format!("{:#?}", pk.get_vk().pinned()),
            r#####"PinnedVerificationKey {
    base_modulus: "0x40000000000000000000000000000000224698fc0994a8dd8c46eb2100000001",
    scalar_modulus: "0x40000000000000000000000000000000224698fc094cf91b992d30ed00000001",
    domain: PinnedEvaluationDomain {
        k: 5,
        extended_k: 7,
        omega: 0x0cc3380dc616f2e1daf29ad1560833ed3baea3393eceb7bc8fa36376929b78cc,
    },
    cs: PinnedConstraintSystem {
        num_fixed_columns: 8,
        num_advice_columns: 5,
        num_instance_columns: 1,
        gates: [
            Sum(
                Sum(
                    Sum(
                        Sum(
                            Product(
                                Advice(
                                    0,
                                ),
                                Fixed(
                                    3,
                                ),
                            ),
                            Product(
                                Advice(
                                    1,
                                ),
                                Fixed(
                                    4,
                                ),
                            ),
                        ),
                        Product(
                            Product(
                                Advice(
                                    0,
                                ),
                                Advice(
                                    1,
                                ),
                            ),
                            Fixed(
                                6,
                            ),
                        ),
                    ),
                    Scaled(
                        Product(
                            Advice(
                                2,
                            ),
                            Fixed(
                                5,
                            ),
                        ),
                        0x40000000000000000000000000000000224698fc094cf91b992d30ed00000000,
                    ),
                ),
                Product(
                    Fixed(
                        2,
                    ),
                    Product(
                        Advice(
                            3,
                        ),
                        Advice(
                            4,
                        ),
                    ),
                ),
            ),
            Product(
                Fixed(
                    7,
                ),
                Sum(
                    Advice(
                        0,
                    ),
                    Scaled(
                        Instance(
                            0,
                        ),
                        0x40000000000000000000000000000000224698fc094cf91b992d30ed00000000,
                    ),
                ),
            ),
        ],
        advice_queries: [
            (
                Column {
                    index: 1,
                    column_type: Advice,
                },
                Rotation(
                    0,
                ),
            ),
            (
                Column {
                    index: 2,
                    column_type: Advice,
                },
                Rotation(
                    0,
                ),
            ),
            (
                Column {
                    index: 3,
                    column_type: Advice,
                },
                Rotation(
                    0,
                ),
            ),
            (
                Column {
                    index: 4,
                    column_type: Advice,
                },
                Rotation(
                    1,
                ),
            ),
            (
                Column {
                    index: 0,
                    column_type: Advice,
                },
                Rotation(
                    -1,
                ),
            ),
        ],
        instance_queries: [
            (
                Column {
                    index: 0,
                    column_type: Instance,
                },
                Rotation(
                    0,
                ),
            ),
        ],
        fixed_queries: [
            (
                Column {
                    index: 6,
                    column_type: Fixed,
                },
                Rotation(
                    0,
                ),
            ),
            (
                Column {
                    index: 7,
                    column_type: Fixed,
                },
                Rotation(
                    0,
                ),
            ),
            (
                Column {
                    index: 0,
                    column_type: Fixed,
                },
                Rotation(
                    0,
                ),
            ),
            (
                Column {
                    index: 2,
                    column_type: Fixed,
                },
                Rotation(
                    0,
                ),
            ),
            (
                Column {
                    index: 3,
                    column_type: Fixed,
                },
                Rotation(
                    0,
                ),
            ),
            (
                Column {
                    index: 4,
                    column_type: Fixed,
                },
                Rotation(
                    0,
                ),
            ),
            (
                Column {
                    index: 1,
                    column_type: Fixed,
                },
                Rotation(
                    0,
                ),
            ),
            (
                Column {
                    index: 5,
                    column_type: Fixed,
                },
                Rotation(
                    0,
                ),
            ),
        ],
        permutations: [
            Argument {
                columns: [
                    Column {
                        index: 1,
                        column_type: Advice,
                    },
                    Column {
                        index: 2,
                        column_type: Advice,
                    },
                    Column {
                        index: 3,
                        column_type: Advice,
                    },
                ],
            },
            Argument {
                columns: [
                    Column {
                        index: 1,
                        column_type: Advice,
                    },
                    Column {
                        index: 2,
                        column_type: Advice,
                    },
                    Column {
                        index: 3,
                        column_type: Advice,
                    },
                ],
            },
        ],
        lookups: [
            Argument {
                input_expressions: [
                    Advice(
                        0,
                    ),
                ],
                table_expressions: [
                    Fixed(
                        0,
                    ),
                ],
            },
            Argument {
                input_expressions: [
                    Product(
                        Advice(
                            0,
                        ),
                        Advice(
                            1,
                        ),
                    ),
                ],
                table_expressions: [
                    Product(
                        Fixed(
                            0,
                        ),
                        Fixed(
                            1,
                        ),
                    ),
                ],
            },
        ],
    },
    fixed_commitments: [
        (0x2bbc94ef7b22aebef24f9a4b0cc1831882548b605171366017d45c3e6fd92075, 0x082b801a6e176239943bfb759fb02138f47a5c8cc4aa7fa0af559fde4e3abd97),
        (0x2bf5082b105b2156ed0e9c5b8e42bf2a240b058f74a464d080e9585274dd1e84, 0x222ad83cee7777e7a160585e212140e5e770dd8d1df788d869b5ee483a5864fb),
        (0x374a656456a0aae7429b23336f825752b575dd5a44290ff614946ee59d6a20c0, 0x054491e187e6e3460e7601fb54ae10836d34d420026f96316f0c5c62f86db9b8),
        (0x374a656456a0aae7429b23336f825752b575dd5a44290ff614946ee59d6a20c0, 0x054491e187e6e3460e7601fb54ae10836d34d420026f96316f0c5c62f86db9b8),
        (0x02e62cd68370b13711139a08cbcdd889e800a272b9ea10acc90880fff9d89199, 0x1a96c468cb0ce77065d3a58f1e55fea9b72d15e44c01bba1e110bd0cbc6e9bc6),
        (0x224ef42758215157d3ee48fb8d769da5bddd35e5929a90a4a89736f5c4b5ae9b, 0x11bc3a1e08eb320cde764f1492ecef956d71e996e2165f7a9a30ad2febb511c1),
        (0x3c145eb1e4f1e49d9eed351a4e2d9f3deed13bc5ba028d3b425084d606418cc8, 0x045d846e7df4e563ce57cd5483d17bad87f0345e18409bf15abc3d71953ae71c),
        (0x27b1cd6c0408a2fe7a764e6ac7abda4f6c7e7a4b3f7375532fe11f3af579de64, 0x19dcda088f6c8ad67408650554cfdd5c8c2e5385cf59c662554c837cf3f42c2d),
    ],
    permutations: [
        VerifyingKey {
            commitments: [
                (0x1347b4b385837977a96b87f199c6a9a81520015539d1e8fa79429bb4ca229a00, 0x2168e404cabef513654d6ff516cde73f0ba87e3dc84e4b940ed675b5f66f3884),
                (0x0e6d69cd2455ec43be640f6397ed65c9e51b1d8c0fd2216339314ff37ade122a, 0x222ed6dc8cfc9ea26dcc10b9d4add791ada60f2b5a63ee1e4635f88aa0c96654),
                (0x13c447846f48c41a5e0675ccf88ebc0cdef2c96c51446d037acb866d24255785, 0x1f0b5414fc5e8219dbfab996eed6129d831488b2386a8b1a63663938903bd63a),
            ],
        },
        VerifyingKey {
            commitments: [
                (0x1347b4b385837977a96b87f199c6a9a81520015539d1e8fa79429bb4ca229a00, 0x2168e404cabef513654d6ff516cde73f0ba87e3dc84e4b940ed675b5f66f3884),
                (0x0e6d69cd2455ec43be640f6397ed65c9e51b1d8c0fd2216339314ff37ade122a, 0x222ed6dc8cfc9ea26dcc10b9d4add791ada60f2b5a63ee1e4635f88aa0c96654),
                (0x13c447846f48c41a5e0675ccf88ebc0cdef2c96c51446d037acb866d24255785, 0x1f0b5414fc5e8219dbfab996eed6129d831488b2386a8b1a63663938903bd63a),
            ],
        },
    ],
}"#####
        );
    }
}
