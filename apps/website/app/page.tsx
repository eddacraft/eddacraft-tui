import { Navbar } from '@/components/navbar';
import { HeroSection } from '@/components/hero-section';
import { ShippingProof } from '@/components/shipping-proof';
import { TrustGap } from '@/components/trust-gap';
import { DecisionIntegrityFlywheel } from '@/components/decision-integrity-flywheel';
import { ProductStages } from '@/components/product-stages';
import { DeliveryBoundary } from '@/components/delivery-boundary';
import { DecisionModel } from '@/components/decision-model';
import { CompanyBand } from '@/components/company-band';
import { CLIFooter } from '@/components/cli-footer';

export default function Home() {
  return (
    <main className="min-h-screen bg-void">
      <Navbar />
      <HeroSection />
      <ShippingProof />
      <TrustGap />
      <DecisionIntegrityFlywheel />
      <ProductStages />
      <DeliveryBoundary />
      <DecisionModel />
      <CompanyBand />
      <CLIFooter />
    </main>
  );
}
