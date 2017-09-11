use quote::{Tokens, Ident};
use syn::{self, Visibility, Field};
use case::CaseExt;
use permutohedron::heap_recursive;
use std::collections::BTreeSet;

use structs::Struct;

pub fn derive(input: &Struct) -> Tokens {
    if let Visibility::Public = input.visibility {
        // nothing, continuing the normal execution
    } else {
        return quote!{}
    }

    let idents = GeneratedIdents::new(input);
    let mut generated = generate_traits(&idents.details_mod);

    generated.append(generate_markers(input, &idents));
    for permutation in all_permutations(&input.fields) {
        generated.append(generate_impl(permutation, &idents));
    }
    generated.append(generate_functions(&idents));

    return generated;
}

fn generate_traits(detail_mod: &Ident) -> Tokens {
     quote! {
        #[allow(dead_code)]
        mod #detail_mod {
            pub trait Zip<'a, Data> {
                type Item: 'a;
                type Iterator: Iterator<Item=Self::Item>;
                fn zip(&'a self, data: Data) -> Self::Iterator;
            }

            pub trait ZipMut<'a, Data> {
                type Item: 'a;
                type Iterator: Iterator<Item=Self::Item>;
                fn zip_mut(&'a mut self, data: Data) -> Self::Iterator;
            }

            // taken from itertools multizip defintion
            #[derive(Clone)]
            pub struct Multizip<T> {
                t: T,
            }

            impl<T> Multizip<T> {
                pub fn new(t: T) -> Multizip<T> {
                    Multizip {
                        t: t
                    }
                }
            }

            macro_rules! impl_zip_iter {
                ($($B:ident),*) => (
                    #[allow(non_snake_case)]
                    #[allow(unused_assignments)]
                    impl<$($B),*> Iterator for Multizip<($($B,)*)>
                        where $( $B: Iterator,)* {
                        type Item = ($($B::Item,)*);

                        fn next(&mut self) -> Option<Self::Item> {
                            let ($(ref mut $B,)*) = self.t;
                            $(
                                let $B = match $B.next() {
                                    None => return None,
                                    Some(elt) => elt
                                };
                            )*
                            Some(($($B,)*))
                        }

                        // TODO: size_hint
                    }

                    #[allow(non_snake_case)]
                    impl<$($B),*> ExactSizeIterator for Multizip<($($B,)*)> where
                        $(
                            $B: ExactSizeIterator,
                        )*
                    { }
                );
            }

            impl_zip_iter!(A);
            impl_zip_iter!(A, B);
            impl_zip_iter!(A, B, C);
            impl_zip_iter!(A, B, C, D);
            impl_zip_iter!(A, B, C, D, E);
            impl_zip_iter!(A, B, C, D, E, F);
            impl_zip_iter!(A, B, C, D, E, F, G);
            impl_zip_iter!(A, B, C, D, E, F, G, H);
        }
    }
}

fn generate_markers(input: &Struct, idents: &GeneratedIdents) -> Tokens {
    let vec_doc_url = format!("[`{0}`](struct.{0}.html)", idents.vec_name);
    let vec_zip_url = format!("[`{0}::zip()`](../struct.{0}.html#method.zip)", idents.vec_name);

    let mut generated = Tokens::new();
    for field in &input.fields {
        let name_str = field.ident.clone().map(|id| id.as_ref().to_owned()).expect("no field name");
        let marker = Ident::from(name_str.to_camel());

        generated.append(quote!{
            /// Marker type to access the `
            #[doc = #name_str]
            /// ` field of a
            #[doc = #vec_doc_url]
            /// in the
            #[doc = #vec_zip_url]
            /// function and familly
            pub struct #marker;
        });
    }

    let markers_mod = &idents.markers_mod;
    quote!{
        pub mod #markers_mod {
            #generated
        }
    }
}

fn generate_impl(permutation: MarkerPermutation, idents: &GeneratedIdents) -> Tokens {
    let details = &idents.details_mod;
    let markers = &idents.markers_mod;

    let markers = permutation.markers(markers);
    let item = permutation.item();
    let iterator = permutation.iterator(details);
    let code = permutation.code(details);

    let vec_name = &idents.vec_name;
    let slice_name = &idents.slice_name;
    let slice_mut_name = &idents.slice_mut_name;

    if permutation.needs_mut() {
        quote!{
            impl<'a> #details::ZipMut<'a, #markers> for #vec_name {
                type Item = #item;
                type Iterator = #iterator;

                fn zip_mut(&'a mut self, _: #markers) -> Self::Iterator {
                    #code
                }
            }

            impl<'a, 'b> #details::ZipMut<'a, #markers> for #slice_mut_name<'b> {
                type Item = #item;
                type Iterator = #iterator;

                fn zip_mut(&'a mut self, _: #markers) -> Self::Iterator {
                    #code
                }
            }
        }
    } else {
        quote!{
            impl<'a> #details::Zip<'a, #markers> for #vec_name {
                type Item = #item;
                type Iterator = #iterator;

                fn zip(&'a self, _: #markers) -> Self::Iterator {
                    #code
                }
            }

            impl<'a, 'b> #details::Zip<'a, #markers> for #slice_name<'b> {
                type Item = #item;
                type Iterator = #iterator;

                fn zip(&'a self, _: #markers) -> Self::Iterator {
                    #code
                }
            }

            impl<'a, 'b> #details::Zip<'a, #markers> for #slice_mut_name<'b> {
                type Item = #item;
                type Iterator = #iterator;

                fn zip(&'a self, _: #markers) -> Self::Iterator {
                    #code
                }
            }
        }
    }
}


fn generate_functions(idents: &GeneratedIdents) -> Tokens {
    let details = &idents.details_mod;

    let vec_name = &idents.vec_name;
    let slice_name = &idents.slice_name;
    let slice_mut_name = &idents.slice_mut_name;

    quote! {
        impl #vec_name {
            pub fn zip<'a, D>(&'a self, data: D) -> <Self as #details::Zip<'a, D>>::Iterator
                where Self: #details::Zip<'a, D>
            {
                <Self as #details::Zip<'a, D>>::zip(self, data)
            }

            pub fn zip_mut<'a, D>(&'a mut self, data: D) -> <Self as #details::ZipMut<'a, D>>::Iterator
                where Self: #details::ZipMut<'a, D>
            {
                <Self as #details::ZipMut<'a, D>>::zip_mut(self, data)
            }
        }

        impl<'b> #slice_name<'b> {
            pub fn zip<'a, D>(&'a self, data: D) -> <Self as #details::Zip<'a, D>>::Iterator
                where Self: #details::Zip<'a, D>
            {
                <Self as #details::Zip<'a, D>>::zip(self, data)
            }
        }

        impl<'b> #slice_mut_name<'b> {
            pub fn zip<'a, D>(&'a self, data: D) -> <Self as #details::Zip<'a, D>>::Iterator
                where Self: #details::Zip<'a, D>
            {
                <Self as #details::Zip<'a, D>>::zip(self, data)
            }

            pub fn zip_mut<'a, D>(&'a mut self, data: D) -> <Self as #details::ZipMut<'a, D>>::Iterator
                where Self: #details::ZipMut<'a, D>
            {
                <Self as #details::ZipMut<'a, D>>::zip_mut(self, data)
            }
        }
    }
}

struct GeneratedIdents {
    pub vec_name: Ident,
    pub slice_name: Ident,
    pub slice_mut_name: Ident,
    pub details_mod: Ident,
    pub markers_mod: Ident,
}

impl GeneratedIdents {
    fn new(input: &Struct) -> GeneratedIdents {
        let details_mod = Ident::from(
            format!("__detail_zip_{}", input.name.as_ref().to_lowercase())
        );
        let markers_mod = Ident::from(
            format!("zip_{}", input.name.as_ref().to_lowercase())
        );
        GeneratedIdents {
            vec_name: input.vec_name(),
            slice_name: input.slice_name(),
            slice_mut_name: input.slice_mut_name(),
            details_mod: details_mod,
            markers_mod: markers_mod,
        }
    }
}

struct MarkerPermutation {
    names: Vec<syn::Ident>,
    types: Vec<syn::Ty>,
    mutables: Vec<bool>,
}

impl MarkerPermutation {
    fn needs_mut(&self) -> bool {
        self.mutables.iter().any(|&x| x)
    }

    fn markers(&self, module: &Ident) -> Tokens {
        let names: Vec<Ident> = self.names
                                    .iter()
                                    .map(|name| name.as_ref()
                                                    .to_camel()
                                                    .into())
                                    .collect();
        let mut markers = Vec::new();
        for (marker, &mu) in names.iter().zip(&self.mutables) {
            markers.push(if mu {
                quote!{&'a mut #module::#marker}
            } else {
                quote!{&'a #module::#marker}
            })
        }

        if markers.len() == 1 {
            markers[0].clone()
        } else {
            quote! {(#(#markers,)*)}
        }
    }

    fn item(&self) -> Tokens {
        let mut types = Vec::new();
        for (ty, &mu) in self.types.iter().zip(&self.mutables) {
            types.push(if mu {
                quote!{&'a mut #ty}
            } else {
                quote!{&'a #ty}
            })
        }

        if types.len() == 1 {
            types[0].clone()
        } else {
            quote! {(#(#types,)*)}
        }
    }

    fn iterator(&self, module: &Ident) -> Tokens {
        let mut types = Vec::new();
        for (ty, &mu) in self.types.iter().zip(&self.mutables) {
            types.push(if mu {
                quote!{::std::slice::IterMut<'a, #ty>}
            } else {
                quote!{::std::slice::Iter<'a, #ty>}
            })
        }

        if types.len() == 1 {
            types[0].clone()
        } else {
            quote! {
                #module::Multizip<(#(#types,)*)>
            }
        }
    }

    fn code(&self, module: &Ident) -> Tokens {
        let mut code = Vec::new();
        for (name, &mu) in self.names.iter().zip(&self.mutables) {
            code.push(if mu {
                quote!{self.#name.iter_mut()}
            } else {
                quote!{self.#name.iter()}
            })
        }

        if code.len() == 1 {
            code[0].clone()
        } else {
            quote! {
                #module::Multizip::new((#(#code,)*))
            }
        }
    }
}

fn all_permutations(fields: &[Field]) -> Vec<MarkerPermutation> {
    let mut all = Vec::new();
    for (i1, f1) in fields.iter().enumerate() {
        for (i2, f2) in fields.iter().enumerate().skip(i1 + 1) {
            for (i3, f3) in fields.iter().enumerate().skip(i2 + 1) {
                for (i4, f4) in fields.iter().enumerate().skip(i3 + 1) {
                    for (i5, f5) in fields.iter().enumerate().skip(i4 + 1) {
                        for (i6, f6) in fields.iter().enumerate().skip(i5 + 1) {
                            for f7 in fields.iter().skip(i6 + 1) {
                                all.append(&mut permutations_for(&[f1, f2, f3, f4, f5, f6, f7]));
                            }
                            all.append(&mut permutations_for(&[f1, f2, f3, f4, f5, f6]));
                        }
                        all.append(&mut permutations_for(&[f1, f2, f3, f4, f5]));
                    }
                    all.append(&mut permutations_for(&[f1, f2, f3, f4]));
                }
                all.append(&mut permutations_for(&[f1, f2, f3]));
            }
            all.append(&mut permutations_for(&[f1, f2]));
        }
        all.append(&mut permutations_for(&[f1]));
    }
    return all;
}

fn permutations_for(fields: &[&Field]) -> Vec<MarkerPermutation> {
    let mut data = fields.iter().map(|field| {
        (field.ident.clone().expect("missing field name"), field.ty.clone())
    }).collect::<Vec<_>>();

    let mut permutations = Vec::new();
    heap_recursive(&mut data, |permutation| {
        for mutables in mutability_permutations(fields.len()) {
            permutations.push(MarkerPermutation {
                names: permutation.iter().cloned().map(|p| p.0).collect(),
                types: permutation.iter().cloned().map(|p| p.1).collect(),
                mutables: mutables,
            })
        }
    });

    return permutations;
}

fn mutability_permutations(n: usize) -> BTreeSet<Vec<bool>> {
    let mut permutations = BTreeSet::new();
    let mut reference = vec![false; n];

    permutations.insert(reference.clone());

    for i in 0..n {
        reference[i] = true;
        heap_recursive(&mut reference.clone(), |permutation| {
            permutations.insert(permutation.to_vec());
        });
    }

    return permutations;
}
